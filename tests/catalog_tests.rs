//! The sigma-racer frame map must stay consistent with the VSS catalog
//! (fixtures synced from sigma-racer-wingman `schemas/`). This is the guard
//! that would have caught the frame-map `vss:` paths that drifted out of the
//! `.vspec`.

#![cfg(feature = "yaml")]

use vss_map::{ValueKind, VssCatalog, VssMap};

const SIGMA_RACER_YAML: &str = include_str!("data/sigma-racer.yaml");
const SIGMA_CLUSTER_VSPEC: &str = include_str!("data/sigma-cluster.vspec");

fn catalog() -> VssCatalog {
    VssCatalog::from_vspec_str(SIGMA_CLUSTER_VSPEC).expect("vspec must parse")
}

#[test]
fn every_mapped_path_exists_in_the_catalog() {
    let map = VssMap::from_frame_map_str(SIGMA_RACER_YAML).expect("frame map must load");
    let issues = map.validate_against(&catalog(), false);
    assert!(
        issues.is_empty(),
        "frame map references paths absent from the VSS catalog:\n{}",
        issues
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn catalog_datatypes_replace_inferred_kinds() {
    let cat = catalog();
    let mut map = VssMap::from_frame_map_str(SIGMA_RACER_YAML).expect("frame map must load");
    map.retype_from_catalog(&cat);

    // Once typed from the catalog, the map and catalog fully agree.
    assert!(map.validate_against(&cat, true).is_empty());

    // Concrete widths the frame map alone could not infer.
    assert_eq!(
        map.lookup("THROTTLE_GEAR", "CurrentGear").unwrap().kind,
        ValueKind::Int8
    );
    assert_eq!(
        map.lookup("ENGINE_STATUS", "CoolantTemp").unwrap().kind,
        ValueKind::Int16
    );
    assert_eq!(
        map.lookup("CHASSIS_ELECTRICAL", "DtcCount").unwrap().kind,
        ValueKind::Uint8
    );
}
