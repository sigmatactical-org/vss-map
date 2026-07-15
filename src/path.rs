//! [`VssPath`].

use core::fmt;
use core::str::FromStr;

/// A validated VSS path: dot-separated identifier segments, e.g.
/// `Vehicle.Powertrain.CombustionEngine.Speed`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VssPath(String);

impl VssPath {
    /// Validate and wrap a path.
    pub fn new(path: impl Into<String>) -> Result<Self, InvalidVssPath> {
        let path = path.into();
        let valid = !path.is_empty()
            && path.split('.').all(|segment| {
                !segment.is_empty()
                    && segment
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '_')
            });
        if valid {
            Ok(Self(path))
        } else {
            Err(InvalidVssPath(path))
        }
    }

    /// The path as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for VssPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for VssPath {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl FromStr for VssPath {
    type Err = InvalidVssPath;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

/// The rejected candidate path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidVssPath(pub String);

impl fmt::Display for InvalidVssPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "not a valid VSS path: {:?}", self.0)
    }
}

impl std::error::Error for InvalidVssPath {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_dotted_identifier_paths() {
        assert!(VssPath::new("Vehicle.Speed").is_ok());
        assert!(VssPath::new("Vehicle.Chassis.Axle.Row1.Wheel.Speed").is_ok());
        assert!(VssPath::new("Vehicle.OBD.MAP").is_ok());
    }

    #[test]
    fn rejects_malformed_paths() {
        assert!(VssPath::new("").is_err());
        assert!(VssPath::new(".Vehicle").is_err());
        assert!(VssPath::new("Vehicle.").is_err());
        assert!(VssPath::new("Vehicle..Speed").is_err());
        assert!(VssPath::new("Vehicle Speed").is_err());
    }
}
