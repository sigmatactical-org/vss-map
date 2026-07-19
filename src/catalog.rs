//! [`VssCatalog`]: the VSS signal tree loaded from a `.vspec` document
//! (`schemas/vss/*.vspec` in sigma-racer-wingman).
//!
//! The catalog is the datatype authority: it says a leaf is `int8` or
//! `boolean` or a `string` with an `allowed` set, plus its unit and range.
//! [`VssMap`] carries which CAN signal lands on which path; validating the map
//! against the catalog catches paths that drifted out of the tree.

use core::fmt;
use std::collections::HashMap;

use serde::Deserialize;

use crate::{InvalidVssPath, MappedSignal, ValueKind, VssMap, VssPath};

/// Whether a leaf is observed, commanded, or static configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeType {
    /// A read-only observed value.
    Sensor,
    /// A commandable value (carries a current and, in a broker, a target).
    Actuator,
    /// Static vehicle configuration.
    Attribute,
}

/// One VSS leaf: its datatype and metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct VssLeaf {
    /// Fully-qualified path.
    pub path: VssPath,
    /// Declared VSS datatype.
    pub datatype: ValueKind,
    /// Sensor / actuator / attribute.
    pub node_type: NodeType,
    /// Engineering unit (`km/h`, `celsius`, ...), when declared.
    pub unit: Option<String>,
    /// Inclusive lower bound, when declared.
    pub min: Option<f64>,
    /// Inclusive upper bound, when declared.
    pub max: Option<f64>,
    /// Permitted string labels for `string` leaves, when declared.
    pub allowed: Option<Vec<String>>,
}

/// The VSS signal tree: path → leaf. Branch nodes are dropped on load.
#[derive(Debug, Clone, Default)]
pub struct VssCatalog {
    leaves: HashMap<String, VssLeaf>,
}

/// Why a `.vspec` document did not load.
#[derive(Debug)]
pub enum CatalogError {
    /// The document is not valid `.vspec` YAML.
    Yaml(serde_yaml_ng::Error),
    /// A node key is not a valid VSS path.
    Path(InvalidVssPath),
    /// A leaf's `datatype` is not a known VSS scalar type.
    UnknownDatatype {
        /// The offending path.
        path: String,
        /// The unrecognized datatype string.
        datatype: String,
    },
    /// A non-branch node has no `datatype`.
    MissingDatatype {
        /// The offending path.
        path: String,
    },
    /// A node's `type` is not `branch`/`sensor`/`actuator`/`attribute`.
    UnknownNodeType {
        /// The offending path.
        path: String,
        /// The unrecognized node type string.
        node_type: String,
    },
}

impl fmt::Display for CatalogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Yaml(e) => write!(f, "vspec YAML: {e}"),
            Self::Path(e) => write!(f, "vspec: {e}"),
            Self::UnknownDatatype { path, datatype } => {
                write!(f, "vspec: {path:?} has unknown datatype {datatype:?}")
            }
            Self::MissingDatatype { path } => write!(f, "vspec: {path:?} has no datatype"),
            Self::UnknownNodeType { path, node_type } => {
                write!(f, "vspec: {path:?} has unknown type {node_type:?}")
            }
        }
    }
}

impl std::error::Error for CatalogError {}

#[derive(Debug, Deserialize)]
struct RawNode {
    #[serde(rename = "type")]
    node_type: String,
    #[serde(default)]
    datatype: Option<String>,
    #[serde(default)]
    unit: Option<String>,
    #[serde(default)]
    min: Option<f64>,
    #[serde(default)]
    max: Option<f64>,
    #[serde(default)]
    allowed: Option<Vec<String>>,
}

fn parse_datatype(s: &str) -> Option<ValueKind> {
    Some(match s {
        "boolean" => ValueKind::Bool,
        "int8" => ValueKind::Int8,
        "int16" => ValueKind::Int16,
        "int32" => ValueKind::Int32,
        "int64" => ValueKind::Int64,
        "uint8" => ValueKind::Uint8,
        "uint16" => ValueKind::Uint16,
        "uint32" => ValueKind::Uint32,
        "uint64" => ValueKind::Uint64,
        "float" => ValueKind::Float,
        "double" => ValueKind::Double,
        "string" => ValueKind::Text,
        _ => return None,
    })
}

impl VssCatalog {
    /// Parse a `.vspec` document (flat dotted-key form) into a catalog.
    pub fn from_vspec_str(vspec: &str) -> Result<Self, CatalogError> {
        let nodes: HashMap<String, RawNode> =
            serde_yaml_ng::from_str(vspec).map_err(CatalogError::Yaml)?;

        let mut leaves = HashMap::new();
        for (path, node) in nodes {
            let node_type = match node.node_type.as_str() {
                "branch" => continue,
                "sensor" => NodeType::Sensor,
                "actuator" => NodeType::Actuator,
                "attribute" => NodeType::Attribute,
                other => {
                    return Err(CatalogError::UnknownNodeType {
                        path,
                        node_type: other.to_owned(),
                    });
                }
            };
            let Some(datatype) = node.datatype.as_deref() else {
                return Err(CatalogError::MissingDatatype { path });
            };
            let Some(datatype) = parse_datatype(datatype) else {
                return Err(CatalogError::UnknownDatatype {
                    datatype: datatype.to_owned(),
                    path,
                });
            };
            let vss_path = VssPath::new(path.clone()).map_err(CatalogError::Path)?;
            leaves.insert(
                path,
                VssLeaf {
                    path: vss_path,
                    datatype,
                    node_type,
                    unit: node.unit,
                    min: node.min,
                    max: node.max,
                    allowed: node.allowed,
                },
            );
        }
        Ok(Self { leaves })
    }

    /// The leaf at `path`, when the catalog defines one.
    pub fn get(&self, path: &str) -> Option<&VssLeaf> {
        self.leaves.get(path)
    }

    /// Whether the catalog defines a leaf at `path`.
    pub fn contains(&self, path: &str) -> bool {
        self.leaves.contains_key(path)
    }

    /// Number of leaves.
    pub fn len(&self) -> usize {
        self.leaves.len()
    }

    /// Whether the catalog has no leaves.
    pub fn is_empty(&self) -> bool {
        self.leaves.is_empty()
    }
}

/// A way a [`VssMap`] disagrees with a [`VssCatalog`].
#[derive(Debug, Clone, PartialEq)]
pub enum CatalogMismatch {
    /// A mapped signal points at a path the catalog does not define.
    UnknownPath {
        /// CAN message name.
        message: String,
        /// CAN signal name.
        signal: String,
        /// The undefined VSS path.
        path: String,
    },
    /// A mapped signal's datatype disagrees with the catalog's.
    DatatypeMismatch {
        /// The VSS path.
        path: String,
        /// Datatype the map carries.
        mapped: ValueKind,
        /// Datatype the catalog declares.
        catalog: ValueKind,
    },
}

impl fmt::Display for CatalogMismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownPath {
                message,
                signal,
                path,
            } => write!(
                f,
                "{message}.{signal} maps to {path:?}, not defined in the catalog"
            ),
            Self::DatatypeMismatch {
                path,
                mapped,
                catalog,
            } => write!(
                f,
                "{path:?} is {mapped:?} in the map but {catalog:?} in the catalog"
            ),
        }
    }
}

impl VssMap {
    /// Check every mapped signal against the catalog. An empty result means
    /// the map is fully covered by (and consistent with) the VSS tree.
    ///
    /// `check_datatypes` also reports leaves whose mapped [`ValueKind`] differs
    /// from the catalog's — off by default because a frame-map-only map infers
    /// coarse kinds (see [`retype_from_catalog`](VssMap::retype_from_catalog)).
    pub fn validate_against(
        &self,
        catalog: &VssCatalog,
        check_datatypes: bool,
    ) -> Vec<CatalogMismatch> {
        let mut issues = Vec::new();
        for (message, signal, mapped) in self.iter() {
            let path = mapped.path.as_str();
            match catalog.get(path) {
                None => issues.push(CatalogMismatch::UnknownPath {
                    message: message.to_owned(),
                    signal: signal.to_owned(),
                    path: path.to_owned(),
                }),
                Some(leaf) if check_datatypes && leaf.datatype != mapped.kind => {
                    issues.push(CatalogMismatch::DatatypeMismatch {
                        path: path.to_owned(),
                        mapped: mapped.kind,
                        catalog: leaf.datatype,
                    })
                }
                Some(_) => {}
            }
        }
        issues
    }

    /// Adopt each mapped signal's datatype from the catalog, making the
    /// catalog the datatype source of truth. Paths absent from the catalog
    /// keep their inferred kind.
    pub fn retype_from_catalog(&mut self, catalog: &VssCatalog) {
        let updates: Vec<(String, String, ValueKind)> = self
            .iter()
            .filter_map(|(message, signal, mapped)| {
                catalog
                    .get(mapped.path.as_str())
                    .map(|leaf| (message.to_owned(), signal.to_owned(), leaf.datatype))
            })
            .collect();
        for (message, signal, datatype) in updates {
            if let Some(MappedSignal { kind, .. }) = self.lookup_mut(&message, &signal) {
                *kind = datatype;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VSPEC: &str = "\
Vehicle:\n  type: branch\n\
Vehicle.Speed:\n  datatype: float\n  type: sensor\n  unit: km/h\n  min: 0\n  max: 400\n\
Vehicle.Powertrain.Transmission.CurrentGear:\n  datatype: int8\n  type: sensor\n  min: -1\n  max: 6\n\
Vehicle.Powertrain.Transmission.PerformanceMode:\n  datatype: string\n  type: sensor\n  allowed: [RAIN, STD, SPORT, TRACK]\n\
Vehicle.ADAS.ABS.IsActive:\n  datatype: boolean\n  type: actuator\n";

    fn catalog() -> VssCatalog {
        VssCatalog::from_vspec_str(VSPEC).expect("vspec must parse")
    }

    #[test]
    fn parses_leaves_and_drops_branches() {
        let cat = catalog();
        assert_eq!(cat.len(), 4); // Vehicle branch dropped
        let speed = cat.get("Vehicle.Speed").unwrap();
        assert_eq!(speed.datatype, ValueKind::Float);
        assert_eq!(speed.node_type, NodeType::Sensor);
        assert_eq!(speed.unit.as_deref(), Some("km/h"));
        assert_eq!(speed.min, Some(0.0));
        assert_eq!(speed.max, Some(400.0));

        let gear = cat
            .get("Vehicle.Powertrain.Transmission.CurrentGear")
            .unwrap();
        assert_eq!(gear.datatype, ValueKind::Int8);

        let mode = cat
            .get("Vehicle.Powertrain.Transmission.PerformanceMode")
            .unwrap();
        assert_eq!(mode.datatype, ValueKind::Text);
        assert_eq!(mode.allowed.as_deref().unwrap().len(), 4);

        let abs = cat.get("Vehicle.ADAS.ABS.IsActive").unwrap();
        assert_eq!(abs.node_type, NodeType::Actuator);
    }

    #[test]
    fn rejects_unknown_datatype() {
        let bad = "Vehicle.X:\n  datatype: quaternion\n  type: sensor\n";
        assert!(matches!(
            VssCatalog::from_vspec_str(bad),
            Err(CatalogError::UnknownDatatype { .. })
        ));
    }

    #[test]
    fn validate_flags_unknown_paths() {
        let cat = catalog();
        let mut map = VssMap::new();
        map.insert(
            "ENGINE_STATUS",
            "EngineRPM",
            "Vehicle.Speed".parse().unwrap(),
            ValueKind::Float,
        );
        // A path the catalog does not define.
        map.insert(
            "CLUSTER_NAV",
            "Heading",
            "Vehicle.CurrentLocation.Heading".parse().unwrap(),
            ValueKind::Float,
        );

        let issues = map.validate_against(&cat, false);
        assert_eq!(issues.len(), 1);
        assert!(matches!(
            &issues[0],
            CatalogMismatch::UnknownPath { path, .. }
                if path == "Vehicle.CurrentLocation.Heading"
        ));
    }

    #[test]
    fn retype_adopts_catalog_datatypes() {
        let cat = catalog();
        let mut map = VssMap::new();
        // Frame-map inference would call a 4-bit gear a Float; the catalog
        // knows it is int8.
        map.insert(
            "THROTTLE_GEAR",
            "CurrentGear",
            "Vehicle.Powertrain.Transmission.CurrentGear"
                .parse()
                .unwrap(),
            ValueKind::Float,
        );
        assert!(!map.validate_against(&cat, true).is_empty());

        map.retype_from_catalog(&cat);
        assert_eq!(
            map.lookup("THROTTLE_GEAR", "CurrentGear").unwrap().kind,
            ValueKind::Int8
        );
        assert!(map.validate_against(&cat, true).is_empty());
    }
}
