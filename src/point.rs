//! [`VssPoint`].

use crate::{VssPath, VssValue};

/// One translated datapoint: a VSS path and its current value.
#[derive(Debug, Clone, PartialEq)]
pub struct VssPoint {
    pub path: VssPath,
    pub value: VssValue,
}
