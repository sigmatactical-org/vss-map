//! Loading the sigma-racer frame map (fixture synced from
//! sigma-racer-wingman `schemas/can/sigma-racer.yaml`).

#![cfg(feature = "yaml")]

use vss_map::{ValueKind, VssMap};

const SIGMA_RACER_YAML: &str = include_str!("data/sigma-racer.yaml");

fn map() -> VssMap {
    VssMap::from_frame_map_str(SIGMA_RACER_YAML).expect("fixture must load")
}

#[test]
fn loads_every_mapped_signal() {
    let map = map();
    // Signals with a non-null `vss:` entry in the fixture.
    assert_eq!(map.len(), 29);
}

#[test]
fn records_frame_ids() {
    let map = map();
    assert_eq!(map.message_for_id(0x0A0), Some("ENGINE_STATUS"));
    assert_eq!(map.message_for_id(0x130), Some("ABS_STATUS"));
    assert_eq!(map.message_for_id(0x230), Some("CLUSTER_NAV"));
    assert_eq!(map.message_for_id(0x7FF), None);
}

#[test]
fn maps_paths_and_infers_kinds() {
    let map = map();

    let rpm = map.lookup("ENGINE_STATUS", "EngineRPM").unwrap();
    assert_eq!(
        rpm.path.as_str(),
        "Vehicle.Powertrain.CombustionEngine.Speed"
    );
    assert_eq!(rpm.kind, ValueKind::Float);

    let redline = map.lookup("ENGINE_STATUS", "Redline").unwrap();
    assert_eq!(redline.kind, ValueKind::Bool);

    let mode = map.lookup("THROTTLE_GEAR", "PerformanceMode").unwrap();
    assert_eq!(mode.kind, ValueKind::Text);

    // `vss: null` signals stay unmapped.
    assert!(map.lookup("THROTTLE_GEAR", "ThrottlePos").is_none());
    assert!(map.lookup("CLUSTER_NAV", "GpsFix").is_none());
}

#[test]
fn exposes_frame_rates_and_timings() {
    let map = map();

    // Rates come straight from the frame map.
    assert_eq!(map.frame_rate_hz(0x0A0), Some(50.0)); // ENGINE_STATUS
    assert_eq!(map.frame_rate_hz(0x220), Some(1.0)); // TRIP_ODOMETER
    assert_eq!(map.frame_rate_hz(0x0E0), Some(0.0)); // RIDE_MODE_REQUEST (event)
    assert_eq!(map.frame_rate_hz(0x999), None);

    let timings = map.frame_timings();
    // Only frames with at least one mapped path appear.
    let engine = timings
        .iter()
        .find(|t| t.id == 0x0A0)
        .expect("ENGINE_STATUS has mapped paths");
    assert_eq!(engine.rate_hz, 50.0);
    assert!(
        engine
            .paths
            .iter()
            .any(|p| p.as_str() == "Vehicle.Powertrain.CombustionEngine.Speed")
    );

    // RIDE_MODE_REQUEST maps nothing (all vss: null) → absent from timings.
    assert!(timings.iter().all(|t| t.id != 0x0E0));
}
