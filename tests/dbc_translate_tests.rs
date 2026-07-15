//! End to end: dbc-rs decode → VSS points, over the real sigma-racer
//! schemas (fixtures synced from sigma-racer-wingman `schemas/can/`).

#![cfg(all(feature = "yaml", feature = "dbc"))]

use vss_map::{VssMap, VssValue, translate_frame};

const SIGMA_RACER_YAML: &str = include_str!("data/sigma-racer.yaml");
const SIGMA_RACER_DBC: &str = include_str!("data/sigma-racer.dbc");

fn fixtures() -> (VssMap, dbc_rs::Dbc) {
    let map = VssMap::from_frame_map_str(SIGMA_RACER_YAML).expect("frame map must load");
    let dbc = dbc_rs::Dbc::parse(SIGMA_RACER_DBC).expect("DBC must parse");
    (map, dbc)
}

fn value_of<'a>(points: &'a [vss_map::VssPoint], path: &str) -> &'a VssValue {
    &points
        .iter()
        .find(|p| p.path.as_str() == path)
        .unwrap_or_else(|| panic!("no point for {path}"))
        .value
}

#[test]
fn engine_status_frame_becomes_vss_points() {
    let (map, dbc) = fixtures();

    // EngineRPM=7450, CoolantTemp=84 °C (raw 124), OilTemp=96 °C (raw 136),
    // Redline=1 at bit 32.
    let payload = [0x1A, 0x1D, 0x7C, 0x88, 0x01, 0x00, 0x00, 0x00];
    let points = translate_frame(&map, &dbc, 0x0A0, &payload, false).unwrap();

    assert_eq!(points.len(), 4);
    assert_eq!(
        value_of(&points, "Vehicle.Powertrain.CombustionEngine.Speed"),
        &VssValue::Float(7450.0)
    );
    assert_eq!(
        value_of(&points, "Vehicle.OBD.CoolantTemperature"),
        &VssValue::Float(84.0)
    );
    assert_eq!(
        value_of(&points, "Vehicle.OBD.OilTemperature"),
        &VssValue::Float(96.0)
    );
    assert_eq!(
        value_of(&points, "Vehicle.Powertrain.CombustionEngine.IsRedline"),
        &VssValue::Bool(true)
    );
}

#[test]
fn value_table_signals_translate_to_labels() {
    let (map, dbc) = fixtures();

    // CurrentGear=3 (bits 16-19), PerformanceMode=3 TRACK (bits 20-22),
    // SideStand=1 (bit 24); ThrottlePos/ThrottleGrip are unmapped.
    let payload = [0x00, 0x00, 0x33, 0x01, 0x00, 0x00, 0x00, 0x00];
    let points = translate_frame(&map, &dbc, 0x0C0, &payload, false).unwrap();

    assert_eq!(points.len(), 3);
    assert_eq!(
        value_of(&points, "Vehicle.Powertrain.Transmission.CurrentGear"),
        &VssValue::Float(3.0)
    );
    assert_eq!(
        value_of(&points, "Vehicle.Powertrain.Transmission.PerformanceMode"),
        &VssValue::Text("TRACK".to_owned())
    );
    assert_eq!(
        value_of(&points, "Vehicle.Body.IsSideStandEngaged"),
        &VssValue::Bool(true)
    );
}

#[test]
fn unknown_frames_translate_to_nothing() {
    let (map, dbc) = fixtures();
    let points = translate_frame(&map, &dbc, 0x7FF, &[0u8; 8], false);
    assert!(points.map(|p| p.is_empty()).unwrap_or(true));
}
