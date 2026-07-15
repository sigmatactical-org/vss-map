//! [`ValueKind`].

use crate::{SignalReading, VssValue};

/// How a mapped signal's physical value becomes a [`VssValue`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueKind {
    /// 1-bit flags → [`VssValue::Bool`].
    Bool,
    /// Value-table signals → the decoded label, falling back to the number
    /// when the source did not resolve one.
    Text,
    /// Everything else → [`VssValue::Float`].
    Float,
}

impl ValueKind {
    /// Convert a reading's physical value according to this kind.
    pub fn coerce(self, reading: &impl SignalReading) -> VssValue {
        match self {
            Self::Bool => VssValue::Bool(reading.value() != 0.0),
            Self::Text => reading
                .label()
                .map(|label| VssValue::Text(label.to_owned()))
                .unwrap_or(VssValue::Float(reading.value())),
            Self::Float => VssValue::Float(reading.value()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coerces_by_kind() {
        assert_eq!(
            ValueKind::Bool.coerce(&("Redline", 1.0)),
            VssValue::Bool(true)
        );
        assert_eq!(
            ValueKind::Float.coerce(&("EngineRPM", 7450.0)),
            VssValue::Float(7450.0)
        );
        // No label available → numeric fallback.
        assert_eq!(
            ValueKind::Text.coerce(&("PerformanceMode", 3.0)),
            VssValue::Float(3.0)
        );
    }
}
