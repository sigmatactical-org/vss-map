//! [`VssDatapoint`] and [`Availability`]: a value with the freshness metadata
//! a live store needs.
//!
//! A [`VssPoint`] is just a path and a value. A `VssDatapoint` adds when the
//! value was produced and whether it is currently trustworthy — the first-class
//! replacement for tracking liveness out of band (wall-clock instants, a
//! synthetic `Vehicle.Service.SignalsLive` path).

use crate::{VssPath, VssPoint, VssValue};

/// Whether a datapoint currently carries a trustworthy value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Availability {
    /// A fresh, valid value.
    #[default]
    Available,
    /// A last-known value that has aged past its freshness window.
    Stale,
    /// No value is being produced (source down, never seen).
    Unavailable,
}

impl Availability {
    /// Classify by age against a time-to-live, both in milliseconds. A value
    /// within `ttl_ms` is [`Available`](Availability::Available); older is
    /// [`Stale`](Availability::Stale).
    pub fn from_age(age_ms: i64, ttl_ms: i64) -> Self {
        if age_ms <= ttl_ms {
            Self::Available
        } else {
            Self::Stale
        }
    }

    /// Whether this is [`Available`](Availability::Available).
    pub fn is_available(self) -> bool {
        matches!(self, Self::Available)
    }
}

/// A VSS value with its production time and availability.
#[derive(Debug, Clone, PartialEq)]
pub struct VssDatapoint {
    /// The VSS path.
    pub path: VssPath,
    /// The current value.
    pub value: VssValue,
    /// Source timestamp in epoch milliseconds, when known. vss-map never reads
    /// the clock itself — the producer supplies this.
    pub timestamp: Option<i64>,
    /// Whether the value is fresh.
    pub availability: Availability,
}

impl VssDatapoint {
    /// An available datapoint with no timestamp.
    pub fn new(path: VssPath, value: VssValue) -> Self {
        Self {
            path,
            value,
            timestamp: None,
            availability: Availability::Available,
        }
    }

    /// Stamp the production time (epoch millis).
    pub fn at(mut self, timestamp: i64) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    /// Set the availability.
    pub fn with_availability(mut self, availability: Availability) -> Self {
        self.availability = availability;
        self
    }

    /// Whether this datapoint is available and, given a `now` and `ttl_ms`, not
    /// older than the freshness window. An unstamped datapoint is judged by its
    /// [`availability`](VssDatapoint::availability) alone.
    pub fn is_fresh(&self, now_ms: i64, ttl_ms: i64) -> bool {
        if !self.availability.is_available() {
            return false;
        }
        match self.timestamp {
            Some(ts) => now_ms.saturating_sub(ts) <= ttl_ms,
            None => true,
        }
    }
}

impl From<VssPoint> for VssDatapoint {
    fn from(point: VssPoint) -> Self {
        Self::new(point.path, point.value)
    }
}

impl From<VssDatapoint> for VssPoint {
    fn from(dp: VssDatapoint) -> Self {
        VssPoint {
            path: dp.path,
            value: dp.value,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point() -> VssPoint {
        VssPoint {
            path: "Vehicle.Speed".parse().unwrap(),
            value: VssValue::Int(84),
        }
    }

    #[test]
    fn availability_classifies_by_age() {
        assert_eq!(Availability::from_age(50, 100), Availability::Available);
        assert_eq!(Availability::from_age(150, 100), Availability::Stale);
    }

    #[test]
    fn datapoint_builds_and_judges_freshness() {
        let dp = VssDatapoint::from(point()).at(1_000);
        assert_eq!(dp.availability, Availability::Available);
        // 1200 - 1000 = 200 <= 500 → fresh.
        assert!(dp.is_fresh(1_200, 500));
        // 2000 - 1000 = 1000 > 500 → stale.
        assert!(!dp.is_fresh(2_000, 500));

        // Explicitly unavailable is never fresh.
        let down = VssDatapoint::from(point()).with_availability(Availability::Unavailable);
        assert!(!down.is_fresh(1_000, 10_000));

        // Unstamped + available → fresh by availability alone.
        assert!(VssDatapoint::from(point()).is_fresh(0, 0));
    }

    #[test]
    fn round_trips_with_point() {
        let dp: VssDatapoint = point().into();
        let back: VssPoint = dp.into();
        assert_eq!(back.path.as_str(), "Vehicle.Speed");
    }
}
