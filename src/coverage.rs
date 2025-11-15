use anyhow::{anyhow, Result};

use crate::dem::{DigitalElevationModel, RadarSite};

const EARTH_RADIUS_M: f64 = 6_371_000.0;

#[derive(Debug, Clone)]
pub struct CoveragePoint {
    pub x_m: f64,
    pub y_m: f64,
    pub ground_elevation_m: f64,
    pub target_altitude_m: f64,
    pub distance_m: f64,
    pub reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CoverageResult {
    pub radar_site: RadarSite,
    pub radar_altitude_m: f64,
    pub min_ground_elevation_m: f64,
    pub target_height_agl_m: f64,
    pub effective_earth_radius_m: f64,
    pub visible: Vec<CoveragePoint>,
    pub occluded: Vec<CoveragePoint>,
}

impl CoverageResult {
    pub fn visibility_ratio(&self) -> f64 {
        let total = self.visible.len() + self.occluded.len();
        if total == 0 {
            0.0
        } else {
            self.visible.len() as f64 / total as f64
        }
    }
}

pub struct RadarCoverageCalculator<'a> {
    dem: &'a DigitalElevationModel,
    radar_site: RadarSite,
    min_ground_elevation_m: f64,
    target_height_agl_m: f64,
    effective_earth_radius_m: f64,
    profile_step_m: f64,
    radar_altitude_m: f64,
}

impl<'a> RadarCoverageCalculator<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        dem: &'a DigitalElevationModel,
        radar_site: RadarSite,
        radar_height_agl_m: Option<f64>,
        target_height_agl_m: f64,
        min_ground_elevation_m: f64,
        effective_earth_radius_scale: f64,
        profile_step_m: Option<f64>,
    ) -> Result<Self> {
        let sample = dem
            .sample(radar_site.x_m, radar_site.y_m)
            .ok_or_else(|| anyhow!("Radar site lies outside DEM extent"))?;
        let radar = RadarSite {
            x_m: radar_site.x_m,
            y_m: radar_site.y_m,
            height_agl_m: radar_height_agl_m.unwrap_or(radar_site.height_agl_m),
        };
        let radar_altitude_m = sample + radar.height_agl_m;
        Ok(Self {
            dem,
            radar_site: radar,
            min_ground_elevation_m,
            target_height_agl_m,
            effective_earth_radius_m: EARTH_RADIUS_M * effective_earth_radius_scale,
            profile_step_m: profile_step_m.unwrap_or(dem.cell_size_m / 2.0),
            radar_altitude_m,
        })
    }

    pub fn compute(&self) -> CoverageResult {
        let mut visible = Vec::new();
        let mut occluded = Vec::new();

        for (x_m, y_m, ground) in self.dem.grid_points() {
            if ground.is_nan() || ground < self.min_ground_elevation_m {
                continue;
            }
            let distance =
                ((x_m - self.radar_site.x_m).powi(2) + (y_m - self.radar_site.y_m).powi(2)).sqrt();
            let target_altitude = ground + self.target_height_agl_m;
            let mut point = CoveragePoint {
                x_m,
                y_m,
                ground_elevation_m: ground,
                target_altitude_m: target_altitude,
                distance_m: distance,
                reason: None,
            };

            let max_distance = self.radio_horizon_range(self.radar_altitude_m, target_altitude);
            if distance > max_distance {
                point.reason = Some("radio_horizon".to_string());
                occluded.push(point);
                continue;
            }

            if self.has_line_of_sight((x_m, y_m), target_altitude) {
                visible.push(point);
            } else {
                point.reason = Some("terrain_block".to_string());
                occluded.push(point);
            }
        }

        CoverageResult {
            radar_site: self.radar_site,
            radar_altitude_m: self.radar_altitude_m,
            min_ground_elevation_m: self.min_ground_elevation_m,
            target_height_agl_m: self.target_height_agl_m,
            effective_earth_radius_m: self.effective_earth_radius_m,
            visible,
            occluded,
        }
    }

    fn has_line_of_sight(&self, target_xy: (f64, f64), target_altitude_m: f64) -> bool {
        let samples = self.dem.profile_between(
            (self.radar_site.x_m, self.radar_site.y_m),
            target_xy,
            Some(self.profile_step_m),
        );
        if samples.is_empty() {
            return false;
        }
        let total_distance = samples.last().map(|(d, _)| *d).unwrap_or(0.0);
        if total_distance == 0.0 {
            return true;
        }
        let delta_altitude = target_altitude_m - self.radar_altitude_m;
        for (idx, (distance_along, terrain_height)) in samples.iter().enumerate() {
            if idx == 0 || idx == samples.len() - 1 {
                continue;
            }
            let los_height =
                self.radar_altitude_m + delta_altitude * (*distance_along / total_distance);
            let adjusted_terrain = terrain_height
                + (distance_along * distance_along) / (2.0 * self.effective_earth_radius_m);
            if adjusted_terrain > los_height {
                return false;
            }
        }
        true
    }

    fn radio_horizon_range(&self, radar_altitude_m: f64, target_altitude_m: f64) -> f64 {
        let radar_term = (2.0 * radar_altitude_m * self.effective_earth_radius_m).sqrt();
        let target_term = (2.0 * target_altitude_m * self.effective_earth_radius_m).sqrt();
        radar_term + target_term
    }
}
