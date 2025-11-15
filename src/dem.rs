use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use ndarray::Array2;
use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Clone, Copy)]
pub struct RadarSite {
    pub x_m: f64,
    pub y_m: f64,
    pub height_agl_m: f64,
}

#[derive(Debug, Error)]
pub enum DemError {
    #[error("failed to read DEM file: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse DEM json: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("DEM grid must be rectangular")]
    NonRectangular,
    #[error("DEM grid must be at least 1x1")]
    Empty,
}

#[derive(Debug, Deserialize)]
struct DemFile {
    origin_x_m: f64,
    origin_y_m: f64,
    cell_size_m: f64,
    elevations: Vec<Vec<f64>>,
    #[serde(default)]
    no_data_value: Option<f64>,
    #[serde(default)]
    description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DigitalElevationModel {
    grid: Array2<f64>,
    pub origin_x_m: f64,
    pub origin_y_m: f64,
    pub cell_size_m: f64,
    pub no_data_value: Option<f64>,
    pub description: Option<String>,
}

impl DigitalElevationModel {
    pub fn from_json_file<P: AsRef<Path>>(path: P) -> Result<Self, DemError> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let data: DemFile = serde_json::from_reader(reader)?;
        Self::from_dem_file(data)
    }

    pub fn from_rows(
        elevations: Vec<Vec<f64>>,
        origin_x_m: f64,
        origin_y_m: f64,
        cell_size_m: f64,
    ) -> Result<Self, DemError> {
        let file = DemFile {
            origin_x_m,
            origin_y_m,
            cell_size_m,
            elevations,
            no_data_value: None,
            description: None,
        };
        Self::from_dem_file(file)
    }

    fn from_dem_file(file: DemFile) -> Result<Self, DemError> {
        let height = file.elevations.len();
        if height == 0 {
            return Err(DemError::Empty);
        }
        let width = file.elevations[0].len();
        if width == 0 {
            return Err(DemError::Empty);
        }
        for row in &file.elevations {
            if row.len() != width {
                return Err(DemError::NonRectangular);
            }
        }
        let flattened: Vec<f64> = file.elevations.into_iter().flatten().collect();
        let grid = Array2::from_shape_vec((height, width), flattened).expect("validated dims");
        Ok(Self {
            grid,
            origin_x_m: file.origin_x_m,
            origin_y_m: file.origin_y_m,
            cell_size_m: file.cell_size_m,
            no_data_value: file.no_data_value,
            description: file.description,
        })
    }

    pub fn width(&self) -> usize {
        self.grid.ncols()
    }

    pub fn height(&self) -> usize {
        self.grid.nrows()
    }

    pub fn extent(&self) -> (f64, f64, f64, f64) {
        let max_x = self.origin_x_m + (self.width() as f64 - 1.0) * self.cell_size_m;
        let max_y = self.origin_y_m + (self.height() as f64 - 1.0) * self.cell_size_m;
        (self.origin_x_m, self.origin_y_m, max_x, max_y)
    }

    pub fn contains(&self, x_m: f64, y_m: f64) -> bool {
        let (min_x, min_y, max_x, max_y) = self.extent();
        x_m >= min_x && x_m <= max_x && y_m >= min_y && y_m <= max_y
    }

    pub fn sample(&self, x_m: f64, y_m: f64) -> Option<f64> {
        let fx = (x_m - self.origin_x_m) / self.cell_size_m;
        let fy = (y_m - self.origin_y_m) / self.cell_size_m;

        if fx < 0.0 || fy < 0.0 {
            return None;
        }
        let max_x = self.width() as f64 - 1.0;
        let max_y = self.height() as f64 - 1.0;
        if fx > max_x || fy > max_y {
            return None;
        }

        let x0 = fx.floor() as usize;
        let y0 = fy.floor() as usize;
        let x1 = x0.min(self.width() - 1);
        let y1 = y0.min(self.height() - 1);
        let dx = fx - x0 as f64;
        let dy = fy - y0 as f64;

        let q11 = self.grid[(y0, x0)];
        let q21 = self.grid[(y0, x1)];
        let q12 = self.grid[(y1, x0)];
        let q22 = self.grid[(y1, x1)];

        if self.is_nodata(q11) || self.is_nodata(q21) || self.is_nodata(q12) || self.is_nodata(q22)
        {
            return None;
        }

        let r1 = (1.0 - dx) * q11 + dx * q21;
        let r2 = (1.0 - dx) * q12 + dx * q22;
        Some((1.0 - dy) * r1 + dy * r2)
    }

    pub fn grid_points(&self) -> GridPointIter<'_> {
        GridPointIter {
            dem: self,
            row: 0,
            col: 0,
        }
    }

    pub fn profile_between(
        &self,
        start: (f64, f64),
        end: (f64, f64),
        step_m: Option<f64>,
    ) -> Vec<(f64, f64)> {
        let (sx, sy) = start;
        let (ex, ey) = end;
        let dx = ex - sx;
        let dy = ey - sy;
        let total_distance = (dx * dx + dy * dy).sqrt();
        if total_distance == 0.0 {
            return self
                .sample(sx, sy)
                .map(|e| vec![(0.0, e)])
                .unwrap_or_default();
        }
        let step = step_m.unwrap_or(self.cell_size_m / 2.0);
        let steps = ((total_distance / step).ceil() as usize).max(2);
        let mut samples = Vec::with_capacity(steps + 1);
        for i in 0..=steps {
            let t = i as f64 / steps as f64;
            let x = sx + t * dx;
            let y = sy + t * dy;
            if let Some(elev) = self.sample(x, y) {
                samples.push((t * total_distance, elev));
            }
        }
        samples
    }

    fn is_nodata(&self, value: f64) -> bool {
        self.no_data_value
            .map(|v| (value - v).abs() < f64::EPSILON)
            .unwrap_or(false)
    }
}

pub struct GridPointIter<'a> {
    dem: &'a DigitalElevationModel,
    row: usize,
    col: usize,
}

impl<'a> Iterator for GridPointIter<'a> {
    type Item = (f64, f64, f64);

    fn next(&mut self) -> Option<Self::Item> {
        if self.row >= self.dem.height() {
            return None;
        }
        let x = self.dem.origin_x_m + self.col as f64 * self.dem.cell_size_m;
        let y = self.dem.origin_y_m + self.row as f64 * self.dem.cell_size_m;
        let value = self.dem.grid[(self.row, self.col)];

        self.col += 1;
        if self.col >= self.dem.width() {
            self.col = 0;
            self.row += 1;
        }

        Some((x, y, value))
    }
}
