from __future__ import annotations

import argparse
import json
import logging
import math
import re
from collections.abc import Sequence
from pathlib import Path
from typing import cast

logger = logging.getLogger(__name__)

METRICS = (
    "ref_cov12",
    "ref_cov24",
    "cand_cov12",
    "cand_cov24",
    "fg_ssim",
    "mask_iou",
)


class AdjustmentVisualEvidenceError(RuntimeError):
    pass


def validate_adjustment_visual_evidence(
    corpus_manifest_path: Path,
    proxy_report_path: Path,
    shape_report_path: Path,
    *,
    minimum_slide_similarity: float = 95.0,
    minimum_shape_ssim: float = 0.75,
) -> dict[str, object]:
    if (
        not math.isfinite(minimum_slide_similarity)
        or not math.isfinite(minimum_shape_ssim)
        or not 0.0 <= minimum_shape_ssim <= 1.0
    ):
        raise AdjustmentVisualEvidenceError(
            "ADJUSTMENT_PROXY_THRESHOLD_INVALID"
        )
    corpus = _as_object(_load_json(corpus_manifest_path), "corpus")
    entries = _as_list(corpus.get("entries"), "corpus.entries")
    expected_slide_count = _as_int(
        corpus.get("slide_count"),
        "corpus.slide_count",
    )
    expected_shapes = {
        _as_str(_as_object(entry, "entry").get("shape_name"), "entry.shape_name")
        for entry in entries
    }
    if len(expected_shapes) != len(entries):
        raise AdjustmentVisualEvidenceError(
            "ADJUSTMENT_CORPUS_SHAPE_DUPLICATE"
        )
    adjustment_pairs = {
        (
            _as_str(_as_object(entry, "entry").get("preset"), "entry.preset"),
            _as_str(_as_object(entry, "entry").get("key"), "entry.key"),
        )
        for entry in entries
    }

    proxy = _as_object(_load_json(proxy_report_path), "proxy")
    declared_slides = _as_int(proxy.get("slide_count"), "proxy.slide_count")
    if declared_slides != expected_slide_count:
        details = f"expected={expected_slide_count}:actual={declared_slides}"
        raise AdjustmentVisualEvidenceError(
            f"ADJUSTMENT_PROXY_SLIDE_COUNT_MISMATCH:{details}"
        )
    similarities = _proxy_similarities(proxy)
    expected_indices = set(range(expected_slide_count))
    if set(similarities) != expected_indices:
        missing = sorted(expected_indices - set(similarities))
        extra = sorted(set(similarities) - expected_indices)
        details = f"missing={missing}:extra={extra}"
        raise AdjustmentVisualEvidenceError(
            f"ADJUSTMENT_PROXY_SLIDE_INVENTORY_MISMATCH:{details}"
        )
    for slide_index, similarity in similarities.items():
        if similarity < minimum_slide_similarity:
            details = f"slide_{slide_index}:{similarity:.6f}"
            raise AdjustmentVisualEvidenceError(
                f"ADJUSTMENT_PROXY_BELOW_THRESHOLD:{details}"
            )

    shape_report = _as_object(_load_json(shape_report_path), "shape_report")
    rows = _as_list(shape_report.get("all_shapes"), "shape_report.all_shapes")
    actual_shapes: dict[str, dict[str, float]] = {}
    for raw_row in rows:
        row = _as_object(raw_row, "shape")
        shape_name = _as_str(row.get("shape_name"), "shape.shape_name")
        if shape_name in actual_shapes:
            raise AdjustmentVisualEvidenceError(
                f"ADJUSTMENT_SHAPE_EVIDENCE_DUPLICATE:{shape_name}"
            )
        actual_shapes[shape_name] = {
            metric: _metric(row.get(metric), f"shape.{metric}")
            for metric in METRICS
        }
    missing_shapes = sorted(expected_shapes - set(actual_shapes))
    if missing_shapes:
        raise AdjustmentVisualEvidenceError(
            f"ADJUSTMENT_SHAPE_EVIDENCE_MISSING:{missing_shapes[0]}"
        )
    extra_shapes = sorted(set(actual_shapes) - expected_shapes)
    if extra_shapes:
        raise AdjustmentVisualEvidenceError(
            f"ADJUSTMENT_SHAPE_EVIDENCE_EXTRA:{extra_shapes[0]}"
        )
    for shape_name, metrics in actual_shapes.items():
        if metrics["fg_ssim"] < minimum_shape_ssim:
            details = f"{shape_name}:{metrics['fg_ssim']:.6f}"
            raise AdjustmentVisualEvidenceError(
                f"ADJUSTMENT_SHAPE_BELOW_THRESHOLD:{details}"
            )
    return {
        "ok": True,
        "adjustment_pair_count": len(adjustment_pairs),
        "shape_count": len(actual_shapes),
        "slide_count": len(similarities),
        "minimum_slide_similarity": min(similarities.values()),
        "minimum_ref_cov12": min(
            metrics["ref_cov12"] for metrics in actual_shapes.values()
        ),
        "minimum_fg_ssim": min(
            metrics["fg_ssim"] for metrics in actual_shapes.values()
        ),
        "minimum_mask_iou": min(
            metrics["mask_iou"] for metrics in actual_shapes.values()
        ),
    }


def _proxy_similarities(proxy: dict[str, object]) -> dict[int, float]:
    result: dict[int, float] = {}
    for raw_slide in _as_list(proxy.get("slides"), "proxy.slides"):
        slide = _as_object(raw_slide, "slide")
        candidate = Path(
            _as_str(slide.get("candidate"), "slide.candidate")
        )
        if candidate.parent.name != "all_adjustments":
            raise AdjustmentVisualEvidenceError(
                f"ADJUSTMENT_PROXY_SLIDE_INVALID:{candidate}"
            )
        match = re.fullmatch(r"slide_(\d+)", candidate.stem)
        if match is None:
            raise AdjustmentVisualEvidenceError(
                f"ADJUSTMENT_PROXY_SLIDE_INVALID:{candidate}"
            )
        slide_index = int(match.group(1))
        if slide_index in result:
            raise AdjustmentVisualEvidenceError(
                f"ADJUSTMENT_PROXY_SLIDE_DUPLICATE:slide_{slide_index}"
            )
        result[slide_index] = _metric(
            slide.get("similarity"),
            "slide.similarity",
        )
    return result


def _metric(value: object, field: str) -> float:
    if not isinstance(value, (int, float)) or isinstance(value, bool):
        raise AdjustmentVisualEvidenceError(
            f"ADJUSTMENT_VISUAL_FIELD_INVALID:{field}"
        )
    result = float(value)
    if not math.isfinite(result):
        raise AdjustmentVisualEvidenceError(
            f"ADJUSTMENT_VISUAL_FIELD_INVALID:{field}"
        )
    return result


def _load_json(path: Path) -> object:
    try:
        return cast(object, json.loads(path.read_text(encoding="utf-8")))
    except (OSError, json.JSONDecodeError) as error:
        raise AdjustmentVisualEvidenceError(
            f"ADJUSTMENT_VISUAL_JSON_INVALID:{path}:{error}"
        ) from error


def _as_object(value: object, field: str) -> dict[str, object]:
    if not isinstance(value, dict):
        raise AdjustmentVisualEvidenceError(
            f"ADJUSTMENT_VISUAL_FIELD_INVALID:{field}"
        )
    return cast(dict[str, object], value)


def _as_list(value: object, field: str) -> list[object]:
    if not isinstance(value, list):
        raise AdjustmentVisualEvidenceError(
            f"ADJUSTMENT_VISUAL_FIELD_INVALID:{field}"
        )
    return cast(list[object], value)


def _as_str(value: object, field: str) -> str:
    if not isinstance(value, str) or not value:
        raise AdjustmentVisualEvidenceError(
            f"ADJUSTMENT_VISUAL_FIELD_INVALID:{field}"
        )
    return value


def _as_int(value: object, field: str) -> int:
    if not isinstance(value, int) or isinstance(value, bool):
        raise AdjustmentVisualEvidenceError(
            f"ADJUSTMENT_VISUAL_FIELD_INVALID:{field}"
        )
    return value


class _Arguments(argparse.Namespace):
    def __init__(self) -> None:
        super().__init__()
        self.corpus_manifest: Path = Path()
        self.proxy_report: Path = Path()
        self.shape_report: Path = Path()
        self.output_json: Path = Path()


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Validate complete adjustment visual evidence."
    )
    _ = parser.add_argument("--corpus-manifest", type=Path, required=True)
    _ = parser.add_argument("--proxy-report", type=Path, required=True)
    _ = parser.add_argument("--shape-report", type=Path, required=True)
    _ = parser.add_argument("--output-json", type=Path, required=True)
    args = cast(_Arguments, parser.parse_args(argv))
    try:
        report = validate_adjustment_visual_evidence(
            args.corpus_manifest,
            args.proxy_report,
            args.shape_report,
        )
    except AdjustmentVisualEvidenceError as error:
        report = {"ok": False, "error": str(error)}
    args.output_json.parent.mkdir(parents=True, exist_ok=True)
    _ = args.output_json.write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    logger.info("Adjustment visual evidence: %s", args.output_json)
    return 0 if report["ok"] else 1


if __name__ == "__main__":
    logging.basicConfig(level=logging.INFO, format="%(levelname)s: %(message)s")
    raise SystemExit(main())
