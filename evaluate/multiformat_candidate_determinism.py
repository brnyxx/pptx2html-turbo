from __future__ import annotations

from collections import Counter
from pathlib import Path
import unicodedata

from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_candidate_artifacts import evidence_binding
from evaluate.multiformat_candidate_sources import CandidateSourceSet
from evaluate.multiformat_candidate_types import CandidateCaptureError, CandidateRun
from evaluate.multiformat_metric_types import MetricError
from evaluate.multiformat_schema import JsonValue, sha256_file
from evaluate.multiformat_strict_json import read_strict_object
from evaluate.multiformat_visual_metrics import _load_png
from evaluate.multiformat_visual_ssim import multiscale_ssim

NATIVE_MINIMUM_MS_SSIM = 84.0
NATIVE_COORDINATE_TOLERANCE = 50.0
NATIVE_TEXT_CHARACTER_OVERLAP = 0.98


class CandidateDeterminismError(CandidateCaptureError):
    pass


def validate_clean_runs(
    source_set: CandidateSourceSet,
    run1: CandidateRun,
    run2: CandidateRun,
) -> None:
    if run1.run_id != 1 or run2.run_id != 2:
        raise CandidateDeterminismError("determinism run IDs must be 1 and 2")
    if run1.browser_version != run2.browser_version:
        raise CandidateDeterminismError("determinism browser versions differ")
    expected = [
        (
            source.track,
            source.source_id,
            source.source_sha256,
            [unit.unit_id for unit in source.units],
        )
        for source in source_set.sources
    ]
    for run in [run1, run2]:
        actual = [
            (
                source.track,
                source.source_id,
                source.source_sha256,
                [unit.unit_id for unit in source.units],
            )
            for source in run.sources
        ]
        if actual != expected:
            raise CandidateDeterminismError("determinism source or unit set differs")
    for left, right in zip(run1.sources, run2.sources, strict=True):
        native = source_set.document_format is not DocumentFormat.PPTX
        html_differs = sha256_file(left.html) != sha256_file(right.html)
        for left_unit, right_unit in zip(left.units, right.units, strict=True):
            png_differs = sha256_file(left_unit.png) != sha256_file(right_unit.png)
            inventory_differs = sha256_file(left_unit.inventory) != sha256_file(
                right_unit.inventory
            )
            if (
                png_differs and not _visually_equivalent(left_unit.png, right_unit.png)
            ) or (
                inventory_differs
                and (
                    not native
                    or not _inventory_equivalent(
                        left_unit.inventory,
                        right_unit.inventory,
                    )
                )
            ):
                raise CandidateDeterminismError("determinism unit artifacts differ")
        if html_differs and not native:
            raise CandidateDeterminismError("determinism HTML differs")


def _visually_equivalent(left: Path, right: Path) -> bool:
    try:
        left_linear, _left_srgb = _load_png(left, "#ffffff")
        right_linear, _right_srgb = _load_png(right, "#ffffff")
        if left_linear.shape != right_linear.shape:
            return False
        return multiscale_ssim(left_linear, right_linear) >= NATIVE_MINIMUM_MS_SSIM
    except (MetricError, OSError, TypeError, ValueError):
        return False


def _inventory_equivalent(left: Path, right: Path) -> bool:
    try:
        return _inventory_values_equivalent(
            read_strict_object(left),
            read_strict_object(right),
        )
    except (OSError, TypeError, UnicodeError, ValueError):
        return False


def _inventory_values_equivalent(
    left: JsonValue,
    right: JsonValue,
    field: str | None = None,
) -> bool:
    if (
        field in {"baseline", "box"}
        and isinstance(left, int | float)
        and not isinstance(left, bool)
        and isinstance(right, int | float)
        and not isinstance(right, bool)
    ):
        return abs(float(left) - float(right)) <= NATIVE_COORDINATE_TOLERANCE
    if isinstance(left, dict) and isinstance(right, dict):
        if set(left) != set(right):
            return False
        return all(
            _text_items_equivalent(left[key], right[key])
            if key == "texts"
            else _inventory_values_equivalent(left[key], right[key], key)
            for key in left
        )
    if isinstance(left, list) and isinstance(right, list):
        return len(left) == len(right) and all(
            _inventory_values_equivalent(
                left_value,
                right_value,
                field,
            )
            for left_value, right_value in zip(left, right, strict=True)
        )
    return left == right


def _text_items_equivalent(left: JsonValue, right: JsonValue) -> bool:
    left_text = _inventory_text(left)
    right_text = _inventory_text(right)
    if left_text is None or right_text is None:
        return False
    if not left_text and not right_text:
        return True
    if not left_text or not right_text:
        return False
    overlap = sum((Counter(left_text) & Counter(right_text)).values())
    return (
        overlap / max(len(left_text), len(right_text)) >= NATIVE_TEXT_CHARACTER_OVERLAP
    )


def _inventory_text(value: JsonValue) -> str | None:
    if not isinstance(value, list):
        return None
    parts: list[str] = []
    for item in value:
        if not isinstance(item, dict) or not isinstance(item.get("value"), str):
            return None
        parts.append(item["value"])
    return "".join(
        character
        for character in unicodedata.normalize("NFC", "".join(parts))
        if not character.isspace()
    )


def determinism_run_value(
    root: Path,
    run: CandidateRun,
) -> dict[str, JsonValue]:
    return {
        "run_id": run.run_id,
        "files": [
            {
                "track": source.track,
                "source_id": source.source_id,
                "source_sha256": source.source_sha256,
                "html": evidence_binding(root, source.html),
                "inventory": evidence_binding(root, source.inventory_manifest),
                "png": [evidence_binding(root, unit.png) for unit in source.units],
            }
            for source in run.sources
        ],
    }
