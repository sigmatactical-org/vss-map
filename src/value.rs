//! [`VssValue`].

/// A translated VSS leaf value.
///
/// Integers are kept distinct from floats so JSON (and any other wire format)
/// round-trips them faithfully: `Vehicle.Powertrain.Transmission.CurrentGear`
/// stays the integer `3`, not `3.0`. The concrete VSS datatype (width,
/// signedness) lives in the [`ValueKind`](crate::ValueKind) that produced the
/// value; the typed accessors below convert to the host field's type.
#[derive(Debug, Clone, PartialEq)]
pub enum VssValue {
    /// Boolean flags (`boolean` signals, 1-bit CAN signals).
    Bool(bool),
    /// Signed integers (`int8`..`int64`).
    Int(i64),
    /// Unsigned integers (`uint8`..`uint64`).
    Uint(u64),
    /// Real numbers (`float`, `double`).
    Float(f64),
    /// Strings and value-table labels ("TRACK", "RAIN", ...).
    Text(String),
}

impl VssValue {
    /// The numeric magnitude, whatever the variant's storage. `Bool` reads as
    /// 0/1; `Text` as its parsed number or `0.0`.
    fn as_f64_lossy(&self) -> f64 {
        match self {
            Self::Bool(b) => *b as u8 as f64,
            Self::Int(i) => *i as f64,
            Self::Uint(u) => *u as f64,
            Self::Float(f) => *f,
            Self::Text(t) => t.parse().unwrap_or(0.0),
        }
    }

    /// The value as an `i64`, rounding floats and coercing bools.
    pub fn as_i64(&self) -> i64 {
        match self {
            Self::Int(i) => *i,
            Self::Uint(u) => (*u).min(i64::MAX as u64) as i64,
            other => other.as_f64_lossy().round() as i64,
        }
    }

    /// The value as a `u64`, clamping negatives to 0.
    pub fn as_u64(&self) -> u64 {
        match self {
            Self::Uint(u) => *u,
            Self::Int(i) => (*i).max(0) as u64,
            other => other.as_f64_lossy().round().max(0.0) as u64,
        }
    }

    /// The value as an `f64`.
    pub fn as_f64(&self) -> f64 {
        self.as_f64_lossy()
    }

    /// The value as an `f32`.
    pub fn as_f32(&self) -> f32 {
        self.as_f64_lossy() as f32
    }

    /// The value as an `i8`, clamped to range.
    pub fn as_i8(&self) -> i8 {
        self.as_i64().clamp(i8::MIN as i64, i8::MAX as i64) as i8
    }

    /// The value as an `i16`, clamped to range.
    pub fn as_i16(&self) -> i16 {
        self.as_i64().clamp(i16::MIN as i64, i16::MAX as i64) as i16
    }

    /// The value as an `i32`, clamped to range.
    pub fn as_i32(&self) -> i32 {
        self.as_i64().clamp(i32::MIN as i64, i32::MAX as i64) as i32
    }

    /// The value as a `u8`, clamped to range.
    pub fn as_u8(&self) -> u8 {
        self.as_u64().min(u8::MAX as u64) as u8
    }

    /// The value as a `u16`, clamped to range.
    pub fn as_u16(&self) -> u16 {
        self.as_u64().min(u16::MAX as u64) as u16
    }

    /// The value as a `u32`, clamped to range.
    pub fn as_u32(&self) -> u32 {
        self.as_u64().min(u32::MAX as u64) as u32
    }

    /// `true` for a set flag; nonzero numbers count as `true`.
    pub fn as_bool(&self) -> bool {
        match self {
            Self::Bool(b) => *b,
            other => other.as_f64_lossy() != 0.0,
        }
    }

    /// The text label, when this is a [`VssValue::Text`].
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Text(t) => Some(t),
            _ => None,
        }
    }
}

#[cfg(feature = "json")]
impl From<&VssValue> for serde_json::Value {
    fn from(value: &VssValue) -> Self {
        match value {
            VssValue::Bool(b) => serde_json::Value::Bool(*b),
            VssValue::Int(i) => serde_json::Value::Number((*i).into()),
            VssValue::Uint(u) => serde_json::Value::Number((*u).into()),
            VssValue::Float(f) => serde_json::Number::from_f64(*f)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
            VssValue::Text(t) => serde_json::Value::String(t.clone()),
        }
    }
}

#[cfg(feature = "json")]
impl VssValue {
    /// Read a JSON value into the closest untyped `VssValue`. Integers keep
    /// their signedness; typed clamping to a specific VSS datatype is
    /// [`ValueKind::coerce_json`](crate::ValueKind::coerce_json).
    pub fn from_json(value: &serde_json::Value) -> Option<Self> {
        use serde_json::Value;
        match value {
            Value::Bool(b) => Some(Self::Bool(*b)),
            Value::String(s) => Some(Self::Text(s.clone())),
            Value::Number(n) => {
                if let Some(u) = n.as_u64() {
                    Some(Self::Uint(u))
                } else if let Some(i) = n.as_i64() {
                    Some(Self::Int(i))
                } else {
                    n.as_f64().map(Self::Float)
                }
            }
            Value::Null | Value::Array(_) | Value::Object(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_accessors_convert_and_clamp() {
        assert_eq!(VssValue::Float(83.6).as_i16(), 84);
        assert_eq!(VssValue::Int(-40).as_i16(), -40);
        assert_eq!(VssValue::Int(-5).as_u8(), 0);
        assert_eq!(VssValue::Int(999).as_u8(), 255);
        assert_eq!(VssValue::Uint(3).as_i8(), 3);
        assert!(VssValue::Float(1.0).as_bool());
        assert_eq!(VssValue::Bool(true).as_i64(), 1);
        assert_eq!(VssValue::Text("TRACK".into()).as_str(), Some("TRACK"));
        assert_eq!(VssValue::Int(3).as_str(), None);
    }

    #[cfg(feature = "json")]
    #[test]
    fn json_round_trips_integers_as_integers() {
        use serde_json::json;
        // The wire must see `3`, never `3.0`.
        assert_eq!(serde_json::Value::from(&VssValue::Int(3)), json!(3));
        assert_eq!(serde_json::Value::from(&VssValue::Uint(255)), json!(255));
        assert_eq!(serde_json::Value::from(&VssValue::Float(13.1)), json!(13.1));

        assert_eq!(VssValue::from_json(&json!(3)), Some(VssValue::Uint(3)));
        assert_eq!(VssValue::from_json(&json!(-3)), Some(VssValue::Int(-3)));
        assert_eq!(VssValue::from_json(&json!(1.5)), Some(VssValue::Float(1.5)));
        assert_eq!(
            VssValue::from_json(&json!(true)),
            Some(VssValue::Bool(true))
        );
        assert_eq!(VssValue::from_json(&json!(null)), None);
    }
}
