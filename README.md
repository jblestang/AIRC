# RadarC (Rust)

RadarC computes radar coverage over a digital elevation model (DEM) while accounting for the
radio horizon. The library exposes a composable API plus a runnable demo that generates an
interactive Plotly map of visible vs occluded terrain cells above a configurable elevation.

## Getting started

```bash
cargo run --bin demo                               # static Plotly HTML output
cargo run --bin viewer --features viewer           # interactive Bevy + egui 3D scene
cargo run --bin viewer --features viewer -- --capture-wireframe   # capture Bevy wireframe screenshot
cargo run --bin wireframe_snapshot                 # headless DEM wireframe PNG
```

The CLI demo reads the synthetic DEM shipped in `data/sample_dem.json`,
places a radar in the southwest corner, and writes `artifacts/coverage_map.html`.
The `viewer` binary loads the same scenario into a 3D scene where you can toggle
visible / occluded points and exaggerate heights through an egui control panel.
The `viewer` target is gated behind the `viewer` cargo feature because its Bevy
dependencies currently require a newer Rust toolchain (≥1.85).

## Features

- Lightweight DEM loader with bilinear interpolation and sampling along paths
- Line-of-sight checks that incorporate Earth curvature (4/3 Earth by default)
- Radio horizon gating that includes target height above terrain
- Plotly visualization helper for quick inspection of visible cells

## Project layout

```
├── Cargo.toml
├── data/sample_dem.json
├── src/
│   ├── lib.rs           # Core crate
│   ├── dem.rs           # DEM utilities
│   ├── coverage.rs      # Visibility logic
│   ├── visualize.rs     # Plotly helpers
│   └── bin/
│       ├── demo.rs              # HTML-exporting CLI
│       ├── viewer.rs            # Bevy + egui interactive app
│       └── wireframe_snapshot.rs # Headless wireframe renderer
└── tests/               # Integration tests
```

## Testing

```bash
cargo test
```
