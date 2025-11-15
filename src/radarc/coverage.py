from __future__ import annotations

from dataclasses import dataclass
from typing import List, Tuple

import numpy as np

from .dem import DigitalElevationModel, RadarSite

_EARTH_RADIUS_M = 6_371_000.0


@dataclass
class CoveragePoint:
    x_m: float
    y_m: float
    ground_elevation_m: float
    target_altitude_m: float
    distance_m: float
    reason: str | None = None


@dataclass
class CoverageResult:
    radar_site: RadarSite
    radar_altitude_m: float
    min_ground_elevation_m: float
    target_height_agl_m: float
    effective_earth_radius_m: float
    visible: List[CoveragePoint]
    occluded: List[CoveragePoint]

    @property
    def visibility_ratio(self) -> float:
        total = len(self.visible) + len(self.occluded)
        if total == 0:
            return 0.0
        return len(self.visible) / total


class RadarCoverageCalculator:
    def __init__(
        self,
        dem: DigitalElevationModel,
        radar_site: RadarSite,
        *,
        radar_height_agl_m: float | None = None,
        target_height_agl_m: float = 0.0,
        min_ground_elevation_m: float = 0.0,
        effective_earth_radius_scale: float = 4.0 / 3.0,
        profile_step_m: float | None = None,
    ) -> None:
        self.dem = dem
        self.radar_site = RadarSite(
            radar_site.x_m,
            radar_site.y_m,
            radar_height_agl_m if radar_height_agl_m is not None else radar_site.height_agl_m,
        )
        self.target_height_agl_m = target_height_agl_m
        self.min_ground_elevation_m = min_ground_elevation_m
        self.effective_earth_radius_m = _EARTH_RADIUS_M * effective_earth_radius_scale
        self.profile_step_m = profile_step_m or dem.cell_size_m / 2

        radar_ground = self.dem.sample(self.radar_site.x_m, self.radar_site.y_m)
        if radar_ground is None:
            raise ValueError("Radar site lies outside of the DEM extent")
        self.radar_altitude_m = radar_ground + self.radar_site.height_agl_m

    def compute(self) -> CoverageResult:
        visible: List[CoveragePoint] = []
        occluded: List[CoveragePoint] = []

        for x_m, y_m, ground in self.dem.grid_points():
            if np.isnan(ground):
                continue
            if ground < self.min_ground_elevation_m:
                continue
            distance = float(np.hypot(x_m - self.radar_site.x_m, y_m - self.radar_site.y_m))
            target_altitude = ground + self.target_height_agl_m
            point = CoveragePoint(
                x_m=x_m,
                y_m=y_m,
                ground_elevation_m=ground,
                target_altitude_m=target_altitude,
                distance_m=distance,
            )

            max_distance = self._radio_horizon_range(self.radar_altitude_m, target_altitude)

            if distance > max_distance:
                point.reason = "radio_horizon"
                occluded.append(point)
                continue

            if self._has_line_of_sight((x_m, y_m), target_altitude):
                visible.append(point)
            else:
                point.reason = point.reason or "terrain_block"
                occluded.append(point)

        return CoverageResult(
            radar_site=self.radar_site,
            radar_altitude_m=self.radar_altitude_m,
            min_ground_elevation_m=self.min_ground_elevation_m,
            target_height_agl_m=self.target_height_agl_m,
            effective_earth_radius_m=self.effective_earth_radius_m,
            visible=visible,
            occluded=occluded,
        )

    def _has_line_of_sight(self, target_xy: Tuple[float, float], target_altitude_m: float) -> bool:
        samples = list(
            self.dem.profile_between(
                (self.radar_site.x_m, self.radar_site.y_m),
                target_xy,
                step_m=self.profile_step_m,
            )
        )
        if not samples:
            return False
        total_distance = samples[-1][0]
        if total_distance == 0:
            return True
        delta_altitude = target_altitude_m - self.radar_altitude_m
        for distance_along, terrain_height in samples[1:-1]:
            los_height = self.radar_altitude_m + (delta_altitude * (distance_along / total_distance))
            adjusted_terrain = terrain_height + (distance_along**2) / (2 * self.effective_earth_radius_m)
            if adjusted_terrain > los_height:
                return False
        return True

    def _radio_horizon_range(self, radar_altitude_m: float, target_altitude_m: float) -> float:
        radar_term = np.sqrt(2.0 * radar_altitude_m * self.effective_earth_radius_m)
        target_term = np.sqrt(2.0 * target_altitude_m * self.effective_earth_radius_m)
        return float(radar_term + target_term)
