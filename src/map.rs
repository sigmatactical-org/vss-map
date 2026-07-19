//! [`VssMap`].

use std::collections::HashMap;

use crate::{MappedSignal, SignalReading, ValueKind, VssPath, VssPoint};

/// The mapping table: which (message, signal) pairs land where in the VSS
/// tree, plus the frame-id → message-name index for id-keyed decoders.
#[derive(Debug, Clone, Default)]
pub struct VssMap {
    // message name → signal name → mapping; nested so lookups probe with
    // borrowed strs instead of allocating a composite key.
    signals: HashMap<String, HashMap<String, MappedSignal>>,
    messages_by_id: HashMap<u32, String>,
}

impl VssMap {
    /// An empty map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Map `message.signal` onto `path`.
    pub fn insert(
        &mut self,
        message: impl Into<String>,
        signal: impl Into<String>,
        path: VssPath,
        kind: ValueKind,
    ) {
        self.signals
            .entry(message.into())
            .or_default()
            .insert(signal.into(), MappedSignal { path, kind });
    }

    /// Record which message name a frame id carries.
    pub fn set_message_id(&mut self, id: u32, message: impl Into<String>) {
        self.messages_by_id.insert(id, message.into());
    }

    /// Message name for a frame id, when known.
    pub fn message_for_id(&self, id: u32) -> Option<&str> {
        self.messages_by_id.get(&id).map(String::as_str)
    }

    /// The mapping entry for `message.signal`, when one exists.
    pub fn lookup(&self, message: &str, signal: &str) -> Option<&MappedSignal> {
        self.signals.get(message)?.get(signal)
    }

    /// The mapping entry for `message.signal`, mutably.
    pub(crate) fn lookup_mut(&mut self, message: &str, signal: &str) -> Option<&mut MappedSignal> {
        self.signals.get_mut(message)?.get_mut(signal)
    }

    /// Every mapped signal as `(message, signal, mapping)`.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str, &MappedSignal)> {
        self.signals.iter().flat_map(|(message, signals)| {
            signals
                .iter()
                .map(move |(signal, mapped)| (message.as_str(), signal.as_str(), mapped))
        })
    }

    /// Translate one reading into a VSS point; `None` when unmapped.
    pub fn translate(&self, message: &str, reading: &impl SignalReading) -> Option<VssPoint> {
        let mapped = self.lookup(message, reading.signal())?;
        Some(VssPoint {
            path: mapped.path.clone(),
            value: mapped.kind.coerce(reading),
        })
    }

    /// Number of mapped signals.
    pub fn len(&self) -> usize {
        self.signals.values().map(HashMap::len).sum()
    }

    /// Whether no signals are mapped.
    pub fn is_empty(&self) -> bool {
        self.signals.values().all(HashMap::is_empty)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::VssValue;

    #[test]
    fn translates_mapped_signals_only() {
        let mut map = VssMap::new();
        map.insert(
            "ENGINE_STATUS",
            "EngineRPM",
            "Vehicle.Powertrain.CombustionEngine.Speed".parse().unwrap(),
            ValueKind::Float,
        );

        let point = map
            .translate("ENGINE_STATUS", &("EngineRPM", 7450.0))
            .unwrap();
        assert_eq!(
            point.path.as_str(),
            "Vehicle.Powertrain.CombustionEngine.Speed"
        );
        assert_eq!(point.value, VssValue::Float(7450.0));

        assert!(map.translate("ENGINE_STATUS", &("Unmapped", 1.0)).is_none());
        assert!(map.translate("OTHER", &("EngineRPM", 1.0)).is_none());
    }

    #[test]
    fn resolves_message_ids() {
        let mut map = VssMap::new();
        map.set_message_id(0x0A0, "ENGINE_STATUS");
        assert_eq!(map.message_for_id(0x0A0), Some("ENGINE_STATUS"));
        assert_eq!(map.message_for_id(0x0C0), None);
    }
}
