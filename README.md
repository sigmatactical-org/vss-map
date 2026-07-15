# vss-map

Glue between decoded vehicle signals and
[COVESA VSS](https://covesa.github.io/vehicle_signal_specification/) paths for
Sigma Racer.

A `VssMap` loads the frame-map YAML documents (`schemas/can/*.yaml` in
[sigma-racer-wingman](https://github.com/sigmatactical-org/sigma-racer-wingman))
and answers where each CAN signal lands in the VSS tree and how its physical
value becomes a VSS leaf value (float, bool, or value-table label).

Decoders stay decoupled through the `SignalReading` trait:

- `dbc` feature — [dbc-rs](https://github.com/sigmatactical-org/dbc-rs)
  decode output implements `SignalReading`, plus `translate_frame` to go
  straight from a CAN frame to VSS points.
- other producers (MDF4 replays, simulators, hand-rolled decoders) implement
  the trait the same way; plain `(&str, f64)` pairs already do.

## Features

| Feature | Default | Purpose |
| --- | --- | --- |
| `yaml` | yes | Load `VssMap` from frame-map YAML documents |
| `dbc` | yes | dbc-rs `SignalReading` impl + `translate_frame` |
| `json` | no | `VssValue` → `serde_json::Value` interop |

## Example

```rust
use vss_map::{VssMap, translate_frame};

let map = VssMap::from_frame_map_str(include_str!("../tests/data/sigma-racer.yaml"))?;
let dbc = dbc_rs::Dbc::parse(include_str!("../tests/data/sigma-racer.dbc"))?;

// ENGINE_STATUS: 7450 rpm, coolant 84 °C, oil 96 °C, redline set.
let payload = [0x1A, 0x1D, 0x7C, 0x88, 0x01, 0x00, 0x00, 0x00];
for point in translate_frame(&map, &dbc, 0x0A0, &payload, false)? {
    println!("{} = {:?}", point.path, point.value);
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

## License

MIT OR Apache-2.0 (see `LICENSE-MIT` / `LICENSE-APACHE`; brand assets are
proprietary, see `BRANDING.md`).
