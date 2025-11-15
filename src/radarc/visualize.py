from __future__ import annotations

from pathlib import Path
from typing import Iterable, List

import plotly.graph_objects as go

from .coverage import CoveragePoint, CoverageResult


def coverage_points_to_records(points: Iterable[CoveragePoint], label: str) -> List[dict]:
    return [
        {
            "category": label,
            "x_m": p.x_m,
            "y_m": p.y_m,
            "ground_elevation_m": p.ground_elevation_m,
            "target_altitude_m": p.target_altitude_m,
            "distance_m": p.distance_m,
            "reason": p.reason,
        }
        for p in points
    ]


def build_coverage_figure(result: CoverageResult) -> go.Figure:
    visible_records = coverage_points_to_records(result.visible, "Visible")
    occluded_records = coverage_points_to_records(result.occluded, "Occluded")

    fig = go.Figure()

    if visible_records:
        fig.add_trace(
            go.Scattergl(
                x=[r["x_m"] for r in visible_records],
                y=[r["y_m"] for r in visible_records],
                mode="markers",
                name="Visible",
                marker=dict(color="#2ecc71", size=6, opacity=0.8),
                customdata=[[r["ground_elevation_m"], r["distance_m"]] for r in visible_records],
                hovertemplate="x=%{x:.0f} m, y=%{y:.0f} m<extra>Visible | elev=%{customdata[0]:.1f} m | dist=%{customdata[1]:.0f} m</extra>",
            )
        )

    if occluded_records:
        fig.add_trace(
            go.Scattergl(
                x=[r["x_m"] for r in occluded_records],
                y=[r["y_m"] for r in occluded_records],
                mode="markers",
                name="Occluded",
                marker=dict(color="#e74c3c", size=5, opacity=0.6),
                customdata=[[r["ground_elevation_m"], r["distance_m"], r["reason"]] for r in occluded_records],
                hovertemplate="x=%{x:.0f} m, y=%{y:.0f} m<extra>Occluded | elev=%{customdata[0]:.1f} m | dist=%{customdata[1]:.0f} m | %{customdata[2]}</extra>",
            )
        )

    fig.add_trace(
        go.Scatter(
            x=[result.radar_site.x_m],
            y=[result.radar_site.y_m],
            mode="markers",
            name="Radar",
            marker=dict(symbol="x", size=12, color="#34495e"),
            hovertemplate="Radar site<extra></extra>",
        )
    )

    fig.update_layout(
        title="Radar coverage projection",
        xaxis_title="Easting (m)",
        yaxis_title="Northing (m)",
        xaxis=dict(scaleanchor="y", scaleratio=1),
        legend=dict(orientation="h", y=-0.2),
        margin=dict(l=20, r=20, t=60, b=40),
    )

    return fig


def save_coverage_map(result: CoverageResult, output_html: str | Path) -> Path:
    fig = build_coverage_figure(result)
    output_path = Path(output_html)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    fig.write_html(output_path, include_plotlyjs="cdn")
    return output_path
