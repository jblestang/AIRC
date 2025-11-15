use anyhow::Result;
use radarc::coverage::RadarCoverageCalculator;
use radarc::dem::{DigitalElevationModel, RadarSite};
use radarc::visualize::save_coverage_map;

fn main() -> Result<()> {
    let dem = DigitalElevationModel::from_json_file("data/sample_dem.json")?;

    let radar_site = RadarSite {
        x_m: 0.0,
        y_m: 0.0,
        height_agl_m: 15.0,
    };

    let calculator =
        RadarCoverageCalculator::new(&dem, radar_site, Some(25.0), 20.0, 150.0, 4.0 / 3.0, None)?;

    let result = calculator.compute();
    println!(
        "Visible cells: {} | Occluded: {} | Ratio: {:.2}%",
        result.visible.len(),
        result.occluded.len(),
        result.visibility_ratio() * 100.0
    );

    let output_path = save_coverage_map(&result, "artifacts/coverage_map.html")?;
    println!("Interactive map written to {}", output_path.display());

    Ok(())
}
