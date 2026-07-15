//! dbc-rs integration: its decoded signals are [`SignalReading`]s.

use crate::{SignalReading, VssMap, VssPoint};

impl SignalReading for dbc_rs::DecodedSignal<'_> {
    fn signal(&self) -> &str {
        self.name
    }

    fn value(&self) -> f64 {
        self.value
    }

    fn label(&self) -> Option<&str> {
        self.description
    }
}

/// Decode one CAN frame with dbc-rs and translate every mapped signal.
///
/// The message name comes from the map's id index first (the map is the
/// authority on naming), falling back to the DBC. Frames the map does not
/// know translate to an empty vec; decode failures bubble up.
pub fn translate_frame(
    map: &VssMap,
    dbc: &dbc_rs::Dbc,
    id: u32,
    payload: &[u8],
    is_extended: bool,
) -> Result<Vec<VssPoint>, dbc_rs::Error> {
    let message = map
        .message_for_id(id)
        .or_else(|| dbc.messages().find_by_id(id).map(|m| m.name()))
        .map(str::to_owned);
    let Some(message) = message else {
        return Ok(Vec::new());
    };
    let decoded = dbc.decode(id, payload, is_extended)?;
    Ok(decoded
        .iter()
        .filter_map(|signal| map.translate(&message, signal))
        .collect())
}
