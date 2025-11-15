use assert_approx_eq::assert_approx_eq;
use radarc::coverage::RadarCoverageCalculator;
use radarc::dem::{DigitalElevationModel, RadarSite};

#[test]
fn dem_bilinear_sampling_matches_expectation() {
    let dem = DigitalElevationModel::from_rows(
        vec![vec![100.0, 110.0], vec![120.0, 130.0]],
        0.0,
        0.0,
        100.0,
    )
    .expect("dem");

    let sample = dem.sample(50.0, 50.0).expect("value");
    assert_approx_eq!(sample, 115.0, 1e-6);
}

#[test]
fn flat_dem_produces_full_visibility() {
    let dem =
        DigitalElevationModel::from_rows(vec![vec![0.0; 5]; 5], 0.0, 0.0, 100.0).expect("dem");
    let radar = RadarSite {
        x_m: 200.0,
        y_m: 200.0,
        height_agl_m: 10.0,
    };

    let calculator = RadarCoverageCalculator::new(&dem, radar, None, 0.0, -10.0, 4.0 / 3.0, None)
        .expect("calculator");
    let result = calculator.compute();
    assert_eq!(result.occluded.len(), 0);
    assert_eq!(result.visible.len(), 25);
}
