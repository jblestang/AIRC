from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, Iterator, Tuple

import numpy as np


@dataclass(frozen=True)
class RadarSite:
    """Location of a radar expressed in a local cartesian grid."""

    x_m: float
    y_m: float
    height_agl_m: float


class DigitalElevationModel:
    """Bilinearly interpolated DEM backed by a numpy grid."""

    def __init__(
        self,
        elevations: np.ndarray,
        *,
        origin_x_m: float,
        origin_y_m: float,
        cell_size_m: float,
        no_data_value: float | None = None,
        description: str | None = None,
    ) -> None:
        if elevations.ndim != 2:
            raise ValueError("elevations must be a 2-D array")
        self._grid = elevations.astype(float)
        self.origin_x_m = float(origin_x_m)
        self.origin_y_m = float(origin_y_m)
        self.cell_size_m = float(cell_size_m)
        self.no_data_value = no_data_value
        self.description = description or ""

    @property
    def shape(self) -> Tuple[int, int]:
        return self._grid.shape

    @property
    def width(self) -> int:
        return self._grid.shape[1]

    @property
    def height(self) -> int:
        return self._grid.shape[0]

    @property
    def extent(self) -> Tuple[float, float, float, float]:
        """Return (min_x, min_y, max_x, max_y)."""
        max_x = self.origin_x_m + (self.width - 1) * self.cell_size_m
        max_y = self.origin_y_m + (self.height - 1) * self.cell_size_m
        return (self.origin_x_m, self.origin_y_m, max_x, max_y)

    @classmethod
    def from_npz(cls, path: str | Path) -> "DigitalElevationModel":
        data = np.load(path)
        grid = data["elevations"]
        origin_x_m = float(data["origin_x_m"])
        origin_y_m = float(data["origin_y_m"])
        cell_size_m = float(data["cell_size_m"])
        description = str(data["description"]) if "description" in data else None
        return cls(
            grid,
            origin_x_m=origin_x_m,
            origin_y_m=origin_y_m,
            cell_size_m=cell_size_m,
            no_data_value=float(data["no_data_value"]) if "no_data_value" in data else None,
            description=description,
        )

    def contains(self, x_m: float, y_m: float) -> bool:
        min_x, min_y, max_x, max_y = self.extent
        return min_x <= x_m <= max_x and min_y <= y_m <= max_y

    def sample(self, x_m: float, y_m: float) -> float | None:
        """Return bilinearly interpolated elevation (meters)."""
        fx = (x_m - self.origin_x_m) / self.cell_size_m
        fy = (y_m - self.origin_y_m) / self.cell_size_m
        if fx < 0 or fy < 0 or fx > self.width - 1 or fy > self.height - 1:
            return None

        x0 = int(np.floor(fx))
        y0 = int(np.floor(fy))
        x1 = min(x0 + 1, self.width - 1)
        y1 = min(y0 + 1, self.height - 1)
        dx = fx - x0
        dy = fy - y0

        q11 = self._grid[y0, x0]
        q21 = self._grid[y0, x1]
        q12 = self._grid[y1, x0]
        q22 = self._grid[y1, x1]

        if self._is_nodata(q11, q21, q12, q22):
            return None

        r1 = (1 - dx) * q11 + dx * q21
        r2 = (1 - dx) * q12 + dx * q22
        return (1 - dy) * r1 + dy * r2

    def _is_nodata(self, *values: float) -> bool:
        if self.no_data_value is None:
            return False
        return any(np.isclose(v, self.no_data_value) for v in values)

    def grid_points(self) -> Iterator[Tuple[float, float, float]]:
        for row in range(self.height):
            y = self.origin_y_m + row * self.cell_size_m
            for col in range(self.width):
                x = self.origin_x_m + col * self.cell_size_m
                yield x, y, self._grid[row, col]

    def profile_between(
        self,
        start: Tuple[float, float],
        end: Tuple[float, float],
        step_m: float | None = None,
    ) -> Iterable[Tuple[float, float]]:
        """Yield (distance_along_m, terrain_height_m)."""
        sx, sy = start
        ex, ey = end
        dx = ex - sx
        dy = ey - sy
        total_distance = float(np.hypot(dx, dy))
        if total_distance == 0:
            elevation = self.sample(sx, sy)
            if elevation is None:
                return []
            return [(0.0, elevation)]

        if step_m is None:
            step_m = self.cell_size_m / 2
        steps = max(int(np.ceil(total_distance / step_m)), 2)
        samples: list[Tuple[float, float]] = []
        for i in range(steps + 1):
            t = i / steps
            x = sx + t * dx
            y = sy + t * dy
            elevation = self.sample(x, y)
            if elevation is None:
                continue
            samples.append((t * total_distance, elevation))
        return samples
