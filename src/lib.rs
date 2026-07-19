//! Glue between decoded vehicle signals and VSS paths.
//!
//! A [`VssMap`] knows which (message, signal) pairs correspond to which
//! [COVESA VSS](https://covesa.github.io/vehicle_signal_specification/) paths
//! and how their physical values become VSS leaf values. Decoders stay
//! decoupled through the [`SignalReading`] trait: dbc-rs decode output
//! implements it behind the `dbc` feature, and any other producer (MDF4
//! replays, simulators, hand-rolled decoders) can implement it the same way.
//!
//! The map itself loads from the sigma-racer frame-map YAML documents
//! (`schemas/can/*.yaml` in sigma-racer-wingman) behind the `yaml` feature.
//!
//! ```
//! use vss_map::{SignalReading, VssMap, VssValue};
//!
//! let mut map = VssMap::new();
//! map.insert(
//!     "ENGINE_STATUS",
//!     "EngineRPM",
//!     "Vehicle.Powertrain.CombustionEngine.Speed".parse().unwrap(),
//!     vss_map::ValueKind::Float,
//! );
//!
//! let point = map.translate("ENGINE_STATUS", &("EngineRPM", 7450.0)).unwrap();
//! assert_eq!(point.path.as_str(), "Vehicle.Powertrain.CombustionEngine.Speed");
//! assert_eq!(point.value, VssValue::Float(7450.0));
//! ```

#![forbid(unsafe_code)]

mod binding;
mod map;
mod mapped_signal;
mod path;
mod point;
mod reading;
mod value;
mod value_kind;

#[cfg(feature = "yaml")]
mod catalog;
#[cfg(feature = "dbc")]
mod dbc;
#[cfg(feature = "yaml")]
mod frame_map;

pub use binding::VssBinding;
pub use map::VssMap;
pub use mapped_signal::MappedSignal;
pub use path::{InvalidVssPath, VssPath};
pub use point::VssPoint;
pub use reading::SignalReading;
pub use value::VssValue;
pub use value_kind::ValueKind;

#[cfg(feature = "yaml")]
pub use catalog::{CatalogError, CatalogMismatch, NodeType, VssCatalog, VssLeaf};
#[cfg(feature = "dbc")]
pub use dbc::translate_frame;
#[cfg(feature = "yaml")]
pub use frame_map::FrameMapError;
