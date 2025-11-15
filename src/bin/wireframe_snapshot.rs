use std::fs::{self, File};
use std::io::BufWriter;
use std::path::Path;

use anyhow::Result;
use plotters::prelude::*;
use png::{BitDepth, ColorType, Encoder};
use radarc::dem::DigitalElevationModel;

const OUTPUT_PATH: &str = "artifacts/dem_wireframe.png";
const IMAGE_SIZE: (u32, u32) = (1600, 1200);
const MARGIN: f64 = 40.0;
const XY_SCALE: f64 = 1.0 / 1500.0;
const Z_SCALE: f64 = 0.004;

fn main() -> Result<()> {
    let dem = DigitalElevationModel::from_json_file("data/sample_dem.json")?;
    render_wireframe(&dem, Path::new(OUTPUT_PATH))?;
    println!("Wireframe screenshot saved to {}", OUTPUT_PATH);
    Ok(())
}

fn render_wireframe(dem: &DigitalElevationModel, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let width = dem.width();
    let height = dem.height();
    let mut elevations = vec![vec![0.0; width]; height];
    let mut min_elev = f64::MAX;
    let mut max_elev = f64::MIN;

    for row in 0..height {
        for col in 0..width {
            let elev = dem.elevation_value(row, col);
            elevations[row][col] = elev;
            min_elev = min_elev.min(elev);
            max_elev = max_elev.max(elev);
        }
    }

    let (min_x, min_y, max_x, max_y) = dem.extent();
    let center_x = (min_x + max_x) / 2.0;
    let center_y = (min_y + max_y) / 2.0;

    let mut projected = vec![vec![(0.0, 0.0); width]; height];
    let mut min_px = f64::MAX;
    let mut max_px = f64::MIN;
    let mut min_py = f64::MAX;
    let mut max_py = f64::MIN;

    for row in 0..height {
        for col in 0..width {
            let world_x = dem.origin_x_m + col as f64 * dem.cell_size_m - center_x;
            let world_y = dem.origin_y_m + row as f64 * dem.cell_size_m - center_y;
            let world_z = elevations[row][col];

            let iso_x = (world_x - world_y) * XY_SCALE;
            let iso_y = (world_x + world_y) * XY_SCALE * 0.55 - (world_z * Z_SCALE);

            projected[row][col] = (iso_x, iso_y);
            min_px = min_px.min(iso_x);
            max_px = max_px.max(iso_x);
            min_py = min_py.min(iso_y);
            max_py = max_py.max(iso_y);
        }
    }

    let span_x = (max_px - min_px).max(1e-6);
    let span_y = (max_py - min_py).max(1e-6);
    let width_px = IMAGE_SIZE.0 as f64;
    let height_px = IMAGE_SIZE.1 as f64;
    let scale_x = (width_px - 2.0 * MARGIN) / span_x;
    let scale_y = (height_px - 2.0 * MARGIN) / span_y;

    let mut screen_points = vec![vec![(0.0, 0.0); width]; height];
    for row in 0..height {
        for col in 0..width {
            let (iso_x, iso_y) = projected[row][col];
            let sx = (iso_x - min_px) * scale_x + MARGIN;
            let sy = (iso_y - min_py) * scale_y + MARGIN;
            screen_points[row][col] = (sx, sy);
        }
    }

    let mut buffer = vec![0u8; (IMAGE_SIZE.0 * IMAGE_SIZE.1 * 3) as usize];
    {
        let root = BitMapBackend::with_buffer(&mut buffer, IMAGE_SIZE).into_drawing_area();
        root.fill(&RGBColor(5, 8, 20))?;

        for row in 0..height {
            for col in 0..width {
                let (sx, sy) = screen_points[row][col];
                if col + 1 < width {
                    let (nx, ny) = screen_points[row][col + 1];
                    let color = wire_color(elevations[row][col], min_elev, max_elev);
                    root.draw(&PathElement::new(vec![coord(sx, sy), coord(nx, ny)], color))?;
                }
                if row + 1 < height {
                    let (nx, ny) = screen_points[row + 1][col];
                    let color = wire_color(elevations[row][col], min_elev, max_elev);
                    root.draw(&PathElement::new(vec![coord(sx, sy), coord(nx, ny)], color))?;
                }
            }
        }

        root.present()?;
    }

    write_png(path, IMAGE_SIZE.0, IMAGE_SIZE.1, &buffer)?;
    Ok(())
}

fn coord(x: f64, y: f64) -> (i32, i32) {
    let flipped_y = (IMAGE_SIZE.1 as f64 - y).max(0.0);
    (x.round() as i32, flipped_y.round() as i32)
}

fn wire_color(value: f64, min: f64, max: f64) -> ShapeStyle {
    let t = if max - min < f64::EPSILON {
        0.5
    } else {
        ((value - min) / (max - min)).clamp(0.0, 1.0)
    };
    let hue = 0.55 - 0.25 * t;
    let rgba = HSLColor(hue, 0.7, 0.55).to_rgba();
    ShapeStyle {
        color: RGBColor(rgba.0, rgba.1, rgba.2).mix(0.9),
        filled: false,
        stroke_width: 1,
    }
}

fn write_png(path: &Path, width: u32, height: u32, buffer: &[u8]) -> Result<()> {
    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    let mut encoder = Encoder::new(writer, width, height);
    encoder.set_color(ColorType::Rgb);
    encoder.set_depth(BitDepth::Eight);
    let mut png_writer = encoder.write_header()?;
    png_writer.write_image_data(buffer)?;
    Ok(())
}
