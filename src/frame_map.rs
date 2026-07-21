//! Load a [`VssMap`] from the sigma-racer frame-map YAML documents
//! (`schemas/can/*.yaml` in sigma-racer-wingman).

use core::fmt;
use std::collections::BTreeMap;

use serde::Deserialize;

use crate::{InvalidVssPath, ValueKind, VssMap, VssPath};

/// Why a frame-map document did not load.
#[derive(Debug)]
pub enum FrameMapError {
    /// The document is not valid frame-map YAML.
    Yaml(serde_yaml_ng::Error),
    /// A `vss:` entry is not a valid VSS path.
    Path(InvalidVssPath),
}

impl fmt::Display for FrameMapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Yaml(e) => write!(f, "frame map YAML: {e}"),
            Self::Path(e) => write!(f, "frame map: {e}"),
        }
    }
}

impl std::error::Error for FrameMapError {}

#[derive(Debug, Deserialize)]
struct FrameMapDoc {
    #[serde(default)]
    frames: Vec<FrameDef>,
}

#[derive(Debug, Deserialize)]
struct FrameDef {
    id: u32,
    name: String,
    #[serde(default)]
    rate_hz: Option<f64>,
    #[serde(default)]
    signals: Vec<SignalDef>,
}

#[derive(Debug, Deserialize)]
struct SignalDef {
    name: String,
    #[serde(default)]
    length: Option<u16>,
    #[serde(default)]
    value_table: Option<BTreeMap<i64, String>>,
    #[serde(default)]
    vss: Option<String>,
}

impl SignalDef {
    /// Value-table signals carry labels; 1-bit signals are flags.
    fn kind(&self) -> ValueKind {
        if self.value_table.is_some() {
            ValueKind::Text
        } else if self.length == Some(1) {
            ValueKind::Bool
        } else {
            ValueKind::Float
        }
    }
}

impl VssMap {
    /// Build a map from one frame-map YAML document.
    pub fn from_frame_map_str(yaml: &str) -> Result<Self, FrameMapError> {
        let mut map = Self::new();
        map.extend_from_frame_map_str(yaml)?;
        Ok(map)
    }

    /// Merge one frame-map YAML document into this map. Signals without a
    /// `vss:` entry are skipped; frame ids are recorded either way.
    pub fn extend_from_frame_map_str(&mut self, yaml: &str) -> Result<(), FrameMapError> {
        let doc: FrameMapDoc = serde_yaml_ng::from_str(yaml).map_err(FrameMapError::Yaml)?;
        for frame in doc.frames {
            self.set_message_id(frame.id, frame.name.clone());
            if let Some(rate_hz) = frame.rate_hz {
                self.set_frame_rate_hz(frame.id, rate_hz);
            }
            for signal in frame.signals {
                let Some(vss) = signal.vss.as_deref() else {
                    continue;
                };
                let path = VssPath::new(vss).map_err(FrameMapError::Path)?;
                let kind = signal.kind();
                self.insert(frame.name.clone(), signal.name, path, kind);
            }
        }
        Ok(())
    }
}
