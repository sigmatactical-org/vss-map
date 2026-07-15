//! [`MappedSignal`].

use crate::{ValueKind, VssPath};

/// Where one CAN signal lands in the VSS tree and how its value converts.
#[derive(Debug, Clone, PartialEq)]
pub struct MappedSignal {
    pub path: VssPath,
    pub kind: ValueKind,
}
