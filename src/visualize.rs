use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use plotly::common::Orientation;
use plotly::common::{HoverInfo, Marker, Mode};
use plotly::layout::{Layout, Legend};
use plotly::{Plot, Scatter, Trace};

use crate::coverage::{CoveragePoint, CoverageResult};

pub fn build_coverage_plot(result: &CoverageResult) -> Plot {
    let mut plot = Plot::new();

    if !result.visible.is_empty() {
        plot.add_trace(build_trace(&result.visible, "Visible", "#2ecc71", 7, false));
    }

    if !result.occluded.is_empty() {
        plot.add_trace(build_trace(
            &result.occluded,
            "Occluded",
            "#e74c3c",
            6,
            true,
        ));
    }

    let radar_trace = Scatter::new(vec![result.radar_site.x_m], vec![result.radar_site.y_m])
        .mode(Mode::Markers)
        .name("Radar")
        .marker(
            Marker::new()
                .symbol(plotly::common::MarkerSymbol::X)
                .size(14)
                .color("#34495e"),
        )
        .hover_text("Radar site")
        .hover_info(HoverInfo::Text);

    plot.add_trace(radar_trace);

    plot.set_layout(
        Layout::new()
            .title("Radar coverage projection")
            .x_axis(plotly::layout::Axis::new().title("Easting (m)"))
            .y_axis(plotly::layout::Axis::new().title("Northing (m)"))
            .legend(Legend::new().orientation(Orientation::Horizontal).y(-0.2)),
    );

    plot
}

fn build_trace(
    points: &[CoveragePoint],
    name: &'static str,
    color: &'static str,
    size: usize,
    include_reason: bool,
) -> Box<dyn Trace> {
    let xs: Vec<f64> = points.iter().map(|p| p.x_m).collect();
    let ys: Vec<f64> = points.iter().map(|p| p.y_m).collect();
    let hover: Vec<String> = points
        .iter()
        .map(|p| {
            if include_reason {
                format!(
                    "{} | elev={:.1} m | dist={:.0} m | {}",
                    name,
                    p.ground_elevation_m,
                    p.distance_m,
                    p.reason.clone().unwrap_or_else(|| "n/a".into())
                )
            } else {
                format!(
                    "{} | elev={:.1} m | dist={:.0} m",
                    name, p.ground_elevation_m, p.distance_m
                )
            }
        })
        .collect();

    Scatter::new(xs, ys)
        .mode(Mode::Markers)
        .name(name)
        .marker(
            Marker::new()
                .size(size)
                .opacity(if include_reason { 0.7 } else { 0.85 })
                .color(color),
        )
        .hover_text_array(hover)
        .hover_info(HoverInfo::Text)
}

pub fn save_coverage_map(
    result: &CoverageResult,
    output_html: impl AsRef<Path>,
) -> Result<PathBuf> {
    let plot = build_coverage_plot(result);
    let path = output_html.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    plot.write_html(path);
    Ok(path.to_path_buf())
}
