from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import TYPE_CHECKING

from evaluate.multiformat_corpus_items import require_keys
from evaluate.multiformat_corpus_types import CorpusError, DocumentFormat
from evaluate.multiformat_evidence import resolve_evidence_path
from evaluate.multiformat_inventory import inventory_boxes, parse_inventory
from evaluate.multiformat_inventory_compare import compare_inventories
from evaluate.multiformat_inventory_types import Box, Inventory
from evaluate.multiformat_metric_formula import score_unit
from evaluate.multiformat_metric_types import (
    MetricError,
    PrimitiveValues,
    UnitScore,
)
from evaluate.multiformat_schema import (
    JsonValue,
    object_value,
    sha256_file,
    sha256_value,
    string_value,
)

if TYPE_CHECKING:
    from evaluate.multiformat_metric_types import VisualScores

_VISUAL_CACHE: dict[
    tuple[str, str, str, tuple[Box, ...]],
    tuple[VisualScores, tuple[int, int]],
] = {}


@dataclass(frozen=True, slots=True)
class UnitArtifacts:
    reference_png: Path
    candidate_png: Path
    reference_inventory: Path
    candidate_inventory: Path

    def paths(self) -> frozenset[Path]:
        return frozenset(
            {
                self.reference_png,
                self.candidate_png,
                self.reference_inventory,
                self.candidate_inventory,
            }
        )


@dataclass(frozen=True, slots=True)
class ComputedUnit:
    score: UnitScore
    artifacts: UnitArtifacts


def compute_unit(
    record: dict[str, JsonValue],
    unit_id: str,
    applicable_metrics: frozenset[str],
    background: str,
    document_format: DocumentFormat,
    evidence_root: Path,
) -> ComputedUnit:
    try:
        artifacts = _parse_artifacts(
            object_value(record, "artifacts"),
            evidence_root,
        )
        if len(artifacts.paths()) != 4:
            raise MetricError("artifact.path", unit_id)
        reference_inventory = parse_inventory(
            artifacts.reference_inventory,
            unit_id,
        )
        candidate_inventory = parse_inventory(
            artifacts.candidate_inventory,
            unit_id,
        )
        visual, reference_size = _cached_visual(
            artifacts,
            background,
            inventory_boxes(reference_inventory),
        )
        if document_format in {DocumentFormat.PPT, DocumentFormat.PPTX} and (
            reference_size != (960, 540)
        ):
            raise MetricError("artifact.dimension", unit_id)
        spreadsheet = document_format in {DocumentFormat.XLS, DocumentFormat.XLSX}
        if (
            spreadsheet and (reference_inventory.texts or candidate_inventory.texts)
        ) or (
            not spreadsheet and (reference_inventory.cells or candidate_inventory.cells)
        ):
            raise MetricError("inventory.identity", unit_id)
        inventory = compare_inventories(
            reference_inventory,
            candidate_inventory,
            spreadsheet=spreadsheet,
        )
        _validate_non_applicable(
            applicable_metrics,
            reference_inventory,
            candidate_inventory,
        )
        primitives = PrimitiveValues(
            ms_ssim=visual.ms_ssim,
            active_tile_ssim=visual.active_tile_ssim,
            color_similarity=visual.color_similarity,
            edge_f1=visual.edge_f1,
            text_or_cell_similarity=(
                inventory.text_or_cell_similarity
                if "content" in applicable_metrics
                else None
            ),
            object_f1=(
                inventory.object_f1 if "content" in applicable_metrics else None
            ),
            matched_box_iou=(
                inventory.matched_box_iou if "layout" in applicable_metrics else None
            ),
            reading_order_similarity=(
                inventory.reading_order_similarity
                if "layout" in applicable_metrics
                else None
            ),
            baseline_similarity=(
                inventory.baseline_similarity
                if "layout" in applicable_metrics
                else None
            ),
        )
        return ComputedUnit(
            score_unit(unit_id, primitives, applicable_metrics),
            artifacts,
        )
    except (CorpusError, OSError, TypeError, ValueError) as error:
        raise MetricError("artifact.schema", unit_id) from error


def _parse_artifacts(
    values: dict[str, JsonValue],
    evidence_root: Path,
) -> UnitArtifacts:
    require_keys(
        values,
        {
            "reference_png",
            "candidate_png",
            "reference_inventory",
            "candidate_inventory",
        },
        "artifact.schema",
    )
    return UnitArtifacts(
        bound_artifact_path(values, "reference_png", evidence_root),
        bound_artifact_path(values, "candidate_png", evidence_root),
        bound_artifact_path(values, "reference_inventory", evidence_root),
        bound_artifact_path(values, "candidate_inventory", evidence_root),
    )


def bound_artifact_path(
    values: dict[str, JsonValue],
    field: str,
    evidence_root: Path,
) -> Path:
    binding = object_value(values, field)
    return resolve_artifact_binding(binding, evidence_root, f"artifact.{field}")


def resolve_artifact_binding(
    binding: dict[str, JsonValue],
    evidence_root: Path,
    reason: str,
) -> Path:
    require_keys(binding, {"path", "sha256"}, reason)
    path = resolve_evidence_path(evidence_root, string_value(binding, "path"))
    if sha256_file(path) != sha256_value(binding, "sha256"):
        raise MetricError("artifact.sha256", reason)
    return path


def _validate_non_applicable(
    applicable_metrics: frozenset[str],
    reference: Inventory,
    candidate: Inventory,
) -> None:
    if "content" not in applicable_metrics and any(
        [
            reference.texts,
            reference.cells,
            reference.objects,
            candidate.texts,
            candidate.cells,
            candidate.objects,
        ]
    ):
        raise MetricError("metric.applicability", "content inventory is not empty")
    if "layout" not in applicable_metrics and any(
        [
            reference.texts,
            reference.cells,
            reference.objects,
            candidate.texts,
            candidate.cells,
            candidate.objects,
        ]
    ):
        raise MetricError("metric.applicability", "layout inventory is not empty")


def _cached_visual(
    artifacts: UnitArtifacts,
    background: str,
    boxes: tuple[Box, ...],
) -> tuple[VisualScores, tuple[int, int]]:
    from evaluate.multiformat_visual_metrics import png_dimensions, score_visual

    reference_hash = sha256_file(artifacts.reference_png)
    candidate_hash = sha256_file(artifacts.candidate_png)
    key = (reference_hash, candidate_hash, background, boxes)
    cached = _VISUAL_CACHE.get(key)
    if cached is not None:
        return cached
    reference_size = png_dimensions(artifacts.reference_png)
    candidate_size = png_dimensions(artifacts.candidate_png)
    if reference_size != candidate_size:
        raise MetricError("artifact.dimension", "native PNG dimensions differ")
    result = score_visual(
        artifacts.reference_png,
        artifacts.candidate_png,
        background,
        boxes,
    )
    cached = result, reference_size
    _VISUAL_CACHE[key] = cached
    return cached
