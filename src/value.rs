//! [`VssValue`].

/// A translated VSS leaf value.
#[derive(Debug, Clone, PartialEq)]
pub enum VssValue {
    /// Boolean flags (1-bit signals).
    Bool(bool),
    /// Numeric sensors and actuators.
    Float(f64),
    /// Value-table labels ("TRACK", "RAIN", ...).
    Text(String),
}

#[cfg(feature = "json")]
impl From<&VssValue> for serde_json::Value {
    fn from(value: &VssValue) -> Self {
        match value {
            VssValue::Bool(b) => serde_json::Value::Bool(*b),
            VssValue::Float(f) => serde_json::Number::from_f64(*f)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
            VssValue::Text(t) => serde_json::Value::String(t.clone()),
        }
    }
}
