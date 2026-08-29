from __future__ import annotations

import math
from dataclasses import dataclass
from pathlib import Path
from typing import Sequence

from PIL import Image

from evaluate.multiformat_candidate_types import CandidateCaptureError
from evaluate.multiformat_schema import JsonValue

TARGET_PRESENTATION_SIZE = (960, 540)
MAX_AGGREGATE_RENDER_WIDTH = 384
MAX_AGGREGATE_RENDER_HEIGHT = 65_535
MAX_AGGREGATE_RENDER_PIXELS = 64_000_000


@dataclass(frozen=True, slots=True)
class AggregatePageGeometry:
    width: int
    height: int
    top: int
    bottom: int
    scaled_width: int
    scaled_top: int
    scaled_bottom: int

    @property
    def scaled_height(self) -> int:
        return self.scaled_bottom - self.scaled_top


@dataclass(frozen=True, slots=True)
class AggregateGeometry:
    width: int
    height: int
    scale: float
    scaled_width: int
    scaled_height: int
    pages: tuple[AggregatePageGeometry, ...]


def aggregate_geometry(dimensions: Sequence[tuple[int, int]]) -> AggregateGeometry:
    if not dimensions or any(width <= 0 or height <= 0 for width, height in dimensions):
        raise ValueError("aggregate dimensions are invalid")
    width = max(page_width for page_width, _ in dimensions)
    height = sum(page_height for _, page_height in dimensions)
    scale = min(
        1.0,
        MAX_AGGREGATE_RENDER_WIDTH / width,
        MAX_AGGREGATE_RENDER_HEIGHT / height,
        math.sqrt(MAX_AGGREGATE_RENDER_PIXELS / (width * height)),
    )
    pages: list[AggregatePageGeometry] = []
    top = 0
    for page_width, page_height in dimensions:
        bottom = top + page_height
        scaled_top = round(top * scale)
        scaled_bottom = round(bottom * scale)
        if scaled_bottom <= scaled_top:
            raise ValueError("aggregate page collapses after scaling")
        pages.append(
            AggregatePageGeometry(
                page_width,
                page_height,
                top,
                bottom,
                scaled_dimension(page_width, scale),
                scaled_top,
                scaled_bottom,
            )
        )
        top = bottom
    return AggregateGeometry(
        width,
        height,
        scale,
        scaled_dimension(width, scale),
        pages[-1].scaled_bottom,
        tuple(pages),
    )


def scaled_dimension(value: float, scale: float) -> int:
    return max(1, round(value * scale))


def canonical_office_page_dimension(value: float) -> int:
    if not math.isfinite(value) or value <= 0:
        raise ValueError("office page dimension is invalid")
    return max(1, math.floor(value * 2.0 + 1e-9))


def browser_version_matches(locked: str, actual: str) -> bool:
    return actual == locked.rsplit(" ", 1)[-1]


def unit_records(value: JsonValue) -> list[dict[str, JsonValue]]:
    if not isinstance(value, list) or any(not isinstance(item, dict) for item in value):
        raise CandidateCaptureError("unit discovery returned an invalid record")
    return value


def record_string(values: dict[str, JsonValue], field: str) -> str:
    value = values.get(field)
    if not isinstance(value, str) or not value:
        raise CandidateCaptureError(f"invalid unit {field}")
    return value


def validate_png(
    path: Path,
    presentation: bool,
    unit: dict[str, JsonValue],
    device_scale: float,
) -> None:
    if presentation:
        expected = TARGET_PRESENTATION_SIZE
    else:
        expected = (
            round(record_number(unit, "width") * device_scale),
            round(record_number(unit, "height") * device_scale),
        )
    with Image.open(path) as image:
        if image.format != "PNG" or image.size != expected:
            raise CandidateCaptureError(
                f"PNG dimension mismatch: expected {expected}, got {image.size}"
            )


def require_presentation_dimensions(unit: dict[str, JsonValue]) -> None:
    actual = (
        round(record_number(unit, "width")),
        round(record_number(unit, "height")),
    )
    if actual != TARGET_PRESENTATION_SIZE:
        raise CandidateCaptureError(
            f"presentation dimensions must be {TARGET_PRESENTATION_SIZE}, got {actual}"
        )


def record_number(values: dict[str, JsonValue], field: str) -> float:
    value = values.get(field)
    if not isinstance(value, int | float) or isinstance(value, bool) or value <= 0:
        raise CandidateCaptureError(f"invalid unit {field}")
    return float(value)
