//! [`VssBinding`]: a bidirectional map between a host state struct and VSS
//! paths, expressed once as per-path get/set closures.
//!
//! This is the seam a producer of full vehicle state (rather than a stream of
//! CAN signals) uses: register each path once with how to read it from the
//! state and how to write it back, and the binding renders snapshots, applies
//! updates, and computes quantized deltas — replacing hand-maintained
//! encode/decode tables that must be kept in sync.
//!
//! ```
//! use vss_map::{VssBinding, VssValue};
//!
//! struct State { speed: f32, gear: i8 }
//!
//! let mut binding = VssBinding::new();
//! binding
//!     .bind(
//!         "Vehicle.Speed",
//!         |s: &State| VssValue::Int(s.speed.round() as i64),
//!         |s, v| s.speed = v.as_f32(),
//!     )
//!     .bind(
//!         "Vehicle.Powertrain.Transmission.CurrentGear",
//!         |s: &State| VssValue::Int(s.gear as i64),
//!         |s, v| s.gear = v.as_i8(),
//!     );
//!
//! let mut state = State { speed: 83.6, gear: 3 };
//! let points = binding.snapshot(&state);
//! assert_eq!(points.len(), 2);
//!
//! binding.apply(&mut state, "Vehicle.Speed", &VssValue::Float(100.0));
//! assert_eq!(state.speed, 100.0);
//! ```

use std::collections::HashMap;

use crate::{VssDatapoint, VssPath, VssPoint, VssValue};

type Getter<S> = Box<dyn Fn(&S) -> VssValue + Send + Sync>;
type Setter<S> = Box<dyn Fn(&mut S, &VssValue) + Send + Sync>;

struct BoundSignal<S> {
    path: VssPath,
    get: Getter<S>,
    set: Setter<S>,
    /// Deadband for [`diff`](VssBinding::diff): numeric changes smaller than
    /// this are not reported. `None` means report any change.
    quantum: Option<f64>,
}

/// A bidirectional binding between host state `S` and VSS paths.
#[derive(Default)]
pub struct VssBinding<S> {
    // Registration order, for stable snapshots.
    order: Vec<String>,
    signals: HashMap<String, BoundSignal<S>>,
}

impl<S> VssBinding<S> {
    /// An empty binding.
    pub fn new() -> Self {
        Self {
            order: Vec::new(),
            signals: HashMap::new(),
        }
    }

    /// Bind `path` to a getter (state → value) and setter (value → state).
    ///
    /// Panics if `path` is not a syntactically valid VSS path — paths are
    /// expected to be string literals fixed at build time. Use
    /// [`validate_against`](VssBinding::validate_against) to check membership
    /// in a catalog.
    pub fn bind(
        &mut self,
        path: &str,
        get: impl Fn(&S) -> VssValue + Send + Sync + 'static,
        set: impl Fn(&mut S, &VssValue) + Send + Sync + 'static,
    ) -> &mut Self {
        self.bind_inner(path, None, Box::new(get), Box::new(set))
    }

    /// Like [`bind`](VssBinding::bind), with a `quantum` deadband applied when
    /// diffing (e.g. `0.1` volts to ignore sensor jitter).
    pub fn bind_quantized(
        &mut self,
        path: &str,
        quantum: f64,
        get: impl Fn(&S) -> VssValue + Send + Sync + 'static,
        set: impl Fn(&mut S, &VssValue) + Send + Sync + 'static,
    ) -> &mut Self {
        self.bind_inner(path, Some(quantum), Box::new(get), Box::new(set))
    }

    fn bind_inner(
        &mut self,
        path: &str,
        quantum: Option<f64>,
        get: Getter<S>,
        set: Setter<S>,
    ) -> &mut Self {
        let vss_path = VssPath::new(path).expect("binding path must be a valid VSS path");
        let key = vss_path.as_str().to_owned();
        if !self.signals.contains_key(&key) {
            self.order.push(key.clone());
        }
        self.signals.insert(
            key,
            BoundSignal {
                path: vss_path,
                get,
                set,
                quantum,
            },
        );
        self
    }

    /// The bound paths, in registration order.
    pub fn paths(&self) -> impl Iterator<Item = &str> {
        self.order.iter().map(String::as_str)
    }

    /// Number of bound paths.
    pub fn len(&self) -> usize {
        self.order.len()
    }

    /// Whether nothing is bound.
    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }

    /// Render the whole state as VSS points, in registration order.
    pub fn snapshot(&self, state: &S) -> Vec<VssPoint> {
        self.order
            .iter()
            .map(|key| {
                let signal = &self.signals[key];
                VssPoint {
                    path: signal.path.clone(),
                    value: (signal.get)(state),
                }
            })
            .collect()
    }

    /// Render the whole state as datapoints stamped with `timestamp` (epoch
    /// millis) and marked available — the freshness-carrying snapshot form.
    pub fn snapshot_at(&self, state: &S, timestamp: i64) -> Vec<VssDatapoint> {
        self.snapshot(state)
            .into_iter()
            .map(|p| VssDatapoint::from(p).at(timestamp))
            .collect()
    }

    /// The points whose value changed between two states, honoring each
    /// binding's `quantum` deadband. Compare `next` against the last state you
    /// emitted so deadbanded drift does not accumulate silently.
    pub fn diff(&self, prev: &S, next: &S) -> Vec<VssPoint> {
        self.order
            .iter()
            .filter_map(|key| {
                let signal = &self.signals[key];
                let before = (signal.get)(prev);
                let after = (signal.get)(next);
                if changed(&before, &after, signal.quantum) {
                    Some(VssPoint {
                        path: signal.path.clone(),
                        value: after,
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    /// Apply one value to the state; unknown paths are ignored. Returns whether
    /// the path was bound.
    pub fn apply(&self, state: &mut S, path: &str, value: &VssValue) -> bool {
        match self.signals.get(path) {
            Some(signal) => {
                (signal.set)(state, value);
                true
            }
            None => false,
        }
    }

    /// Check every bound path against a catalog, reusing
    /// [`VssMap`](crate::VssMap)-style validation. Requires the `yaml` feature.
    #[cfg(feature = "yaml")]
    pub fn validate_against(&self, catalog: &crate::VssCatalog) -> Vec<String> {
        self.order
            .iter()
            .filter(|path| !catalog.contains(path))
            .cloned()
            .collect()
    }
}

/// Whether `after` differs from `before` beyond an optional numeric deadband.
fn changed(before: &VssValue, after: &VssValue, quantum: Option<f64>) -> bool {
    match quantum {
        Some(q) if q > 0.0 => (after.as_f64() - before.as_f64()).abs() >= q,
        _ => before != after,
    }
}

#[cfg(feature = "json")]
impl<S> VssBinding<S> {
    /// Render the whole state as VSS path → JSON value entries (the wire /
    /// snapshot form).
    pub fn to_json_map(&self, state: &S) -> HashMap<String, serde_json::Value> {
        self.snapshot(state)
            .into_iter()
            .map(|p| (p.path.as_str().to_owned(), (&p.value).into()))
            .collect()
    }

    /// The changed entries between two states as JSON (the delta form).
    pub fn diff_json_map(&self, prev: &S, next: &S) -> HashMap<String, serde_json::Value> {
        self.diff(prev, next)
            .into_iter()
            .map(|p| (p.path.as_str().to_owned(), (&p.value).into()))
            .collect()
    }

    /// Apply one JSON entry to the state; unknown paths are ignored.
    pub fn apply_json(&self, state: &mut S, path: &str, value: &serde_json::Value) -> bool {
        match VssValue::from_json(value) {
            Some(v) => self.apply(state, path, &v),
            None => false,
        }
    }

    /// Apply a batch of JSON entries.
    pub fn apply_json_map(&self, state: &mut S, data: &HashMap<String, serde_json::Value>) {
        for (path, value) in data {
            self.apply_json(state, path, value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct State {
        speed: f32,
        gear: i8,
        battery_v: f32,
        mode: String,
    }

    fn binding() -> VssBinding<State> {
        let mut b = VssBinding::new();
        b.bind(
            "Vehicle.Speed",
            |s: &State| VssValue::Int(s.speed.round() as i64),
            |s, v| s.speed = v.as_f32(),
        )
        .bind(
            "Vehicle.Powertrain.Transmission.CurrentGear",
            |s: &State| VssValue::Int(s.gear as i64),
            |s, v| s.gear = v.as_i8(),
        )
        .bind_quantized(
            "Vehicle.ElectricalSystem.Battery.Voltage",
            0.1,
            |s: &State| VssValue::Float(s.battery_v as f64),
            |s, v| s.battery_v = v.as_f32(),
        )
        .bind(
            "Vehicle.Powertrain.Transmission.PerformanceMode",
            |s: &State| VssValue::Text(s.mode.clone()),
            |s, v| {
                if let Some(t) = v.as_str() {
                    s.mode = t.to_owned();
                }
            },
        );
        b
    }

    #[test]
    fn snapshot_is_stable_and_complete() {
        let b = binding();
        let s = State {
            speed: 83.6,
            gear: 3,
            battery_v: 13.1,
            mode: "SPORT".into(),
        };
        let points = b.snapshot(&s);
        assert_eq!(points.len(), 4);
        assert_eq!(points[0].path.as_str(), "Vehicle.Speed");
        assert_eq!(points[0].value, VssValue::Int(84));
    }

    #[test]
    fn apply_round_trips_through_setters() {
        let b = binding();
        let mut s = State::default();
        b.apply(&mut s, "Vehicle.Speed", &VssValue::Float(100.4));
        b.apply(
            &mut s,
            "Vehicle.Powertrain.Transmission.CurrentGear",
            &VssValue::Int(4),
        );
        assert_eq!(s.speed, 100.4);
        assert_eq!(s.gear, 4);
        // Unknown path is a no-op.
        assert!(!b.apply(&mut s, "Vehicle.Nope", &VssValue::Int(1)));
    }

    #[test]
    fn diff_deadbands_by_quantum() {
        let b = binding();
        let base = State {
            battery_v: 13.10,
            ..Default::default()
        };
        // Below the 0.1 V deadband → not reported.
        let jitter = State {
            battery_v: 13.14,
            ..Default::default()
        };
        assert!(b.diff(&base, &jitter).is_empty());

        // Beyond the deadband → reported.
        let real = State {
            battery_v: 13.30,
            ..Default::default()
        };
        let d = b.diff(&base, &real);
        assert_eq!(d.len(), 1);
        assert_eq!(
            d[0].path.as_str(),
            "Vehicle.ElectricalSystem.Battery.Voltage"
        );
    }

    #[test]
    fn diff_reports_exact_changes_without_quantum() {
        let b = binding();
        let a = State {
            gear: 2,
            ..Default::default()
        };
        let c = State {
            gear: 3,
            ..Default::default()
        };
        let d = b.diff(&a, &c);
        assert_eq!(d.len(), 1);
        assert_eq!(
            d[0].path.as_str(),
            "Vehicle.Powertrain.Transmission.CurrentGear"
        );
    }

    #[cfg(feature = "json")]
    #[test]
    fn json_map_round_trips() {
        use serde_json::json;
        let b = binding();
        let s = State {
            speed: 42.0,
            gear: 2,
            battery_v: 12.9,
            mode: "RAIN".into(),
        };
        let map = b.to_json_map(&s);
        assert_eq!(map["Vehicle.Speed"], json!(42));
        assert_eq!(
            map["Vehicle.Powertrain.Transmission.PerformanceMode"],
            json!("RAIN")
        );

        let mut back = State::default();
        b.apply_json_map(&mut back, &map);
        assert_eq!(back.speed, 42.0);
        assert_eq!(back.mode, "RAIN");
    }
}
