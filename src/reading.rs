//! [`SignalReading`].

/// A decoded physical signal, however it was produced.
///
/// This is the seam between decoders and the VSS mapping: dbc-rs decode
/// output implements it behind the `dbc` feature, and other producers (MDF4
/// replays, simulators, hand-rolled decoders) implement it the same way.
pub trait SignalReading {
    /// Signal name as spelled in the frame map (DBC signal name).
    fn signal(&self) -> &str;

    /// Physical value after scaling.
    fn value(&self) -> f64;

    /// Value-table label when the source decoded one ("TRACK", "Drive", ...).
    fn label(&self) -> Option<&str> {
        None
    }
}

/// Plain `(signal, value)` pairs are readings — handy for tests and sources
/// without value tables.
impl SignalReading for (&str, f64) {
    fn signal(&self) -> &str {
        self.0
    }

    fn value(&self) -> f64 {
        self.1
    }
}
