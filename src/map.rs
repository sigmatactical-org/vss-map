//! [`VssMap`].

use std::collections::HashMap;

use crate::{MappedSignal, SignalReading, ValueKind, VssPath, VssPoint};

/// How often a frame is transmitted, and which VSS paths it carries — the
/// inputs a producer needs to judge per-signal freshness. `rate_hz == 0` marks
/// an event-driven frame (no periodic budget).
#[derive(Debug, Clone, PartialEq)]
pub struct FrameTiming {
    /// CAN frame id.
    pub id: u32,
    /// Message name.
    pub message: String,
    /// Nominal transmit rate in hertz; `0.0` for event-driven frames.
    pub rate_hz: f64,
    /// The mapped VSS paths this frame feeds.
    pub paths: Vec<VssPath>,
}

/// The mapping table: which (message, signal) pairs land where in the VSS
/// tree, plus the frame-id → message-name index for id-keyed decoders.
#[derive(Debug, Clone, Default)]
pub struct VssMap {
    // message name → signal name → mapping; nested so lookups probe with
    // borrowed strs instead of allocating a composite key.
    signals: HashMap<String, HashMap<String, MappedSignal>>,
    messages_by_id: HashMap<u32, String>,
    // frame id → nominal transmit rate (hz); 0.0 == event-driven.
    frame_rate_hz: HashMap<u32, f64>,
}

impl VssMap {
    /// An empty map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Map `message.signal` onto `path`.
    pub fn insert(
        &mut self,
        message: impl Into<String>,
        signal: impl Into<String>,
        path: VssPath,
        kind: ValueKind,
    ) {
        self.signals
            .entry(message.into())
            .or_default()
            .insert(signal.into(), MappedSignal { path, kind });
    }

    /// Record which message name a frame id carries.
    pub fn set_message_id(&mut self, id: u32, message: impl Into<String>) {
        self.messages_by_id.insert(id, message.into());
    }

    /// Message name for a frame id, when known.
    pub fn message_for_id(&self, id: u32) -> Option<&str> {
        self.messages_by_id.get(&id).map(String::as_str)
    }

    /// Record a frame's nominal transmit rate (hz); `0.0` for event-driven.
    pub fn set_frame_rate_hz(&mut self, id: u32, rate_hz: f64) {
        self.frame_rate_hz.insert(id, rate_hz);
    }

    /// A frame's nominal transmit rate (hz), when known.
    pub fn frame_rate_hz(&self, id: u32) -> Option<f64> {
        self.frame_rate_hz.get(&id).copied()
    }

    /// Per-frame timing and the VSS paths each frame feeds — the inputs a
    /// producer uses to track per-signal freshness. Only frames with at least
    /// one mapped path are returned; order is unspecified.
    pub fn frame_timings(&self) -> Vec<FrameTiming> {
        self.messages_by_id
            .iter()
            .filter_map(|(&id, message)| {
                let paths: Vec<VssPath> = self
                    .signals
                    .get(message)?
                    .values()
                    .map(|m| m.path.clone())
                    .collect();
                if paths.is_empty() {
                    return None;
                }
                Some(FrameTiming {
                    id,
                    message: message.clone(),
                    rate_hz: self.frame_rate_hz.get(&id).copied().unwrap_or(0.0),
                    paths,
                })
            })
            .collect()
    }

    /// The mapping entry for `message.signal`, when one exists.
    pub fn lookup(&self, message: &str, signal: &str) -> Option<&MappedSignal> {
        self.signals.get(message)?.get(signal)
    }

    /// The mapping entry for `message.signal`, mutably.
    pub(crate) fn lookup_mut(&mut self, message: &str, signal: &str) -> Option<&mut MappedSignal> {
        self.signals.get_mut(message)?.get_mut(signal)
    }

    /// Every mapped signal as `(message, signal, mapping)`.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str, &MappedSignal)> {
        self.signals.iter().flat_map(|(message, signals)| {
            signals
                .iter()
                .map(move |(signal, mapped)| (message.as_str(), signal.as_str(), mapped))
        })
    }

    /// Translate one reading into a VSS point; `None` when unmapped.
    pub fn translate(&self, message: &str, reading: &impl SignalReading) -> Option<VssPoint> {
        let mapped = self.lookup(message, reading.signal())?;
        Some(VssPoint {
            path: mapped.path.clone(),
            value: mapped.kind.coerce(reading),
        })
    }

    /// Number of mapped signals.
    pub fn len(&self) -> usize {
        self.signals.values().map(HashMap::len).sum()
    }

    /// Whether no signals are mapped.
    pub fn is_empty(&self) -> bool {
        self.signals.values().all(HashMap::is_empty)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::VssValue;

    #[test]
    fn translates_mapped_signals_only() {
        let mut map = VssMap::new();
        map.insert(
            "ENGINE_STATUS",
            "EngineRPM",
            "Vehicle.Powertrain.CombustionEngine.Speed".parse().unwrap(),
            ValueKind::Float,
        );

        let point = map
            .translate("ENGINE_STATUS", &("EngineRPM", 7450.0))
            .unwrap();
        assert_eq!(
            point.path.as_str(),
            "Vehicle.Powertrain.CombustionEngine.Speed"
        );
        assert_eq!(point.value, VssValue::Float(7450.0));

        assert!(map.translate("ENGINE_STATUS", &("Unmapped", 1.0)).is_none());
        assert!(map.translate("OTHER", &("EngineRPM", 1.0)).is_none());
    }

    #[test]
    fn resolves_message_ids() {
        let mut map = VssMap::new();
        map.set_message_id(0x0A0, "ENGINE_STATUS");
        assert_eq!(map.message_for_id(0x0A0), Some("ENGINE_STATUS"));
        assert_eq!(map.message_for_id(0x0C0), None);
    }
}
