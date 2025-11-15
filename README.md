# RadarC

RadarC computes radar coverage over a digital elevation model (DEM) while accounting for the radio horizon.
It ships with a lightweight synthetic DEM so you can experiment immediately, and provides primitives to

- load gridded elevation data,
- derive line-of-sight visibility with earth-curvature compensation,
- visualize the visible cells that exceed a configurable ground-elevation threshold.

## Quick start

```bash
python -m venv .venv
source .venv/bin/activate
pip install -e .
python examples/run_demo.py
```

The demo places a notional radar on the southwest corner of the sample terrain. It writes an interactive Plotly
HTML file at `artifacts/coverage_map.html` showing which cells higher than 150 m above mean sea level are visible.

## Project layout

```
├── data/                 # Sample DEM assets
├── examples/             # Executable scripts
├── src/radarc/           # Library code
└── tests/                # Unit tests
```

## Configuration knobs

The core entry point is `radarc.coverage.RadarCoverageCalculator`. Key parameters:

- `radar_height_agl_m`: sensor height above ground.
- `min_ground_elevation_m`: minimum elevation to highlight (e.g. tall structures, ridgelines).
- `target_height_agl_m`: optional additional height for targets above terrain.
- `effective_earth_radius_scale`: set to 1.333 for the standard 4/3 earth assumption.

The calculator returns a structured `CoverageResult` that includes the visible grid cells, rejected cells, and
metadata that the visualization layer can consume.

## Testing

```bash
pytest
```
