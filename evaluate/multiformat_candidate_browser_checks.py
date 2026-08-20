from __future__ import annotations

from pathlib import Path

from PIL import Image

from evaluate.multiformat_candidate_types import CandidateCaptureError
from evaluate.multiformat_schema import JsonValue

TARGET_PRESENTATION_SIZE = (960, 540)


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
