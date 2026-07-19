//! [`ValueKind`].

use crate::{SignalReading, VssValue};

/// A VSS leaf datatype: how a signal's physical value becomes a [`VssValue`],
/// and how a JSON value coerces back. Mirrors the COVESA VSS scalar datatypes
/// (`boolean`, `int8`..`int64`, `uint8`..`uint64`, `float`, `double`,
/// `string`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueKind {
    /// `boolean` (and 1-bit CAN signals) → [`VssValue::Bool`].
    Bool,
    /// `int8` → [`VssValue::Int`], clamped to range.
    Int8,
    /// `int16` → [`VssValue::Int`], clamped to range.
    Int16,
    /// `int32` → [`VssValue::Int`], clamped to range.
    Int32,
    /// `int64` → [`VssValue::Int`].
    Int64,
    /// `uint8` → [`VssValue::Uint`], clamped to range.
    Uint8,
    /// `uint16` → [`VssValue::Uint`], clamped to range.
    Uint16,
    /// `uint32` → [`VssValue::Uint`], clamped to range.
    Uint32,
    /// `uint64` → [`VssValue::Uint`].
    Uint64,
    /// `float` (32-bit) → [`VssValue::Float`].
    Float,
    /// `double` (64-bit) → [`VssValue::Float`].
    Double,
    /// `string` / value-table label → [`VssValue::Text`], falling back to the
    /// number when the source did not resolve a label.
    Text,
}

impl ValueKind {
    /// Build the value carried by an already-numeric magnitude, clamping
    /// integers to the datatype's range. `Text` here keeps the numeric
    /// fallback used when no label is available.
    fn coerce_magnitude(self, v: VssValue) -> VssValue {
        match self {
            Self::Bool => VssValue::Bool(v.as_bool()),
            Self::Int8 => VssValue::Int(v.as_i8() as i64),
            Self::Int16 => VssValue::Int(v.as_i16() as i64),
            Self::Int32 => VssValue::Int(v.as_i32() as i64),
            Self::Int64 => VssValue::Int(v.as_i64()),
            Self::Uint8 => VssValue::Uint(v.as_u8() as u64),
            Self::Uint16 => VssValue::Uint(v.as_u16() as u64),
            Self::Uint32 => VssValue::Uint(v.as_u32() as u64),
            Self::Uint64 => VssValue::Uint(v.as_u64()),
            Self::Float | Self::Double => VssValue::Float(v.as_f64()),
            Self::Text => VssValue::Float(v.as_f64()),
        }
    }

    /// Convert a decoder reading's physical value according to this kind.
    /// `Text` prefers the decoded label and falls back to the number.
    pub fn coerce(self, reading: &impl SignalReading) -> VssValue {
        if self == Self::Text
            && let Some(label) = reading.label()
        {
            return VssValue::Text(label.to_owned());
        }
        self.coerce_magnitude(VssValue::Float(reading.value()))
    }

    /// Coerce a JSON value into this datatype (the apply/decode direction:
    /// wire → typed value), clamping integers and preserving string labels.
    #[cfg(feature = "json")]
    pub fn coerce_json(self, value: &serde_json::Value) -> VssValue {
        if self == Self::Text {
            return match value.as_str() {
                Some(s) => VssValue::Text(s.to_owned()),
                None => VssValue::from_json(value).unwrap_or(VssValue::Float(0.0)),
            };
        }
        let magnitude = VssValue::from_json(value).unwrap_or(VssValue::Float(0.0));
        self.coerce_magnitude(magnitude)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coerces_readings_by_kind() {
        assert_eq!(
            ValueKind::Bool.coerce(&("Redline", 1.0)),
            VssValue::Bool(true)
        );
        assert_eq!(
            ValueKind::Float.coerce(&("EngineRPM", 7450.0)),
            VssValue::Float(7450.0)
        );
        // Integers clamp to their width and stay integers.
        assert_eq!(ValueKind::Int8.coerce(&("Gear", 3.0)), VssValue::Int(3));
        assert_eq!(
            ValueKind::Uint8.coerce(&("Dtc", 300.0)),
            VssValue::Uint(255)
        );
        assert_eq!(
            ValueKind::Int16.coerce(&("Coolant", 84.0)),
            VssValue::Int(84)
        );
        // No label available → numeric fallback.
        assert_eq!(
            ValueKind::Text.coerce(&("PerformanceMode", 3.0)),
            VssValue::Float(3.0)
        );
    }

    #[cfg(feature = "json")]
    #[test]
    fn coerces_json_by_kind() {
        use serde_json::json;
        assert_eq!(ValueKind::Int16.coerce_json(&json!(84)), VssValue::Int(84));
        assert_eq!(ValueKind::Int8.coerce_json(&json!(999)), VssValue::Int(127));
        assert_eq!(ValueKind::Uint8.coerce_json(&json!(-4)), VssValue::Uint(0));
        assert_eq!(
            ValueKind::Bool.coerce_json(&json!(true)),
            VssValue::Bool(true)
        );
        assert_eq!(
            ValueKind::Text.coerce_json(&json!("TRACK")),
            VssValue::Text("TRACK".into())
        );
        assert_eq!(
            ValueKind::Float.coerce_json(&json!(13.1)),
            VssValue::Float(13.1)
        );
    }
}
