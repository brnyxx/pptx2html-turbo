from __future__ import annotations

from dataclasses import dataclass
from decimal import ROUND_HALF_EVEN, Decimal
from enum import StrEnum
from pathlib import Path
from typing import TypedDict

from evaluate.multiformat_corpus_types import DocumentFormat, SecurityOutcome

SIX_PLACES = Decimal("0.000001")


@dataclass(frozen=True, slots=True)
class MetricsEvidenceBindings:
    """Everything a metrics validation decision binds to besides the bytes."""

    contract_path: Path
    corpus_path: Path
    evaluator_manifest_sha256: str
    oracle_lock_sha256: str
    evidence_root: Path
    oracle_lock_path: Path | None


class MetricStatus(StrEnum):
    READY = "READY"
    INCOMPLETE = "INCOMPLETE"


class MetricError(Exception):
    __slots__ = ("reason", "detail")

    def __init__(self, reason: str, detail: str) -> None:
        self.reason = reason
        self.detail = detail
        super().__init__(reason, detail)

    def __str__(self) -> str:
        return f"{self.reason}: {self.detail}"


@dataclass(frozen=True, slots=True)
class PrimitiveValues:
    ms_ssim: Decimal | None
    active_tile_ssim: Decimal | None
    color_similarity: Decimal | None
    edge_f1: Decimal | None
    text_or_cell_similarity: Decimal | None
    object_f1: Decimal | None
    matched_box_iou: Decimal | None
    reading_order_similarity: Decimal | None
    baseline_similarity: Decimal | None


@dataclass(frozen=True, slots=True)
class VisualScores:
    ms_ssim: Decimal
    active_tile_ssim: Decimal
    color_similarity: Decimal
    edge_f1: Decimal


@dataclass(frozen=True, slots=True)
class UnitScore:
    unit_id: str
    visual: Decimal
    content: Decimal
    layout: Decimal
    score: Decimal


@dataclass(frozen=True, slots=True)
class ConformanceUnitSpec:
    source_id: str
    source_sha256: str
    unit_id: str
    ordinal: int
    stratum: str
    applicable_metrics: frozenset[str]
    background: str


@dataclass(frozen=True, slots=True)
class BlindFileSpec:
    source_id: str
    source_sha256: str
    unit_count: int
    applicable_metrics: frozenset[str]
    background: str


@dataclass(frozen=True, slots=True)
class SecurityCaseSpec:
    source_id: str
    source_sha256: str
    case_family: str
    expected_outcome: SecurityOutcome


@dataclass(frozen=True, slots=True)
class CorpusMetricSpec:
    document_format: DocumentFormat
    conformance: dict[str, ConformanceUnitSpec]
    blind: dict[str, BlindFileSpec]
    security: dict[str, SecurityCaseSpec]

    def pair_ids(self) -> frozenset[str]:
        blind_ids = {
            f"{source_id}-unit-{ordinal}"
            for source_id, spec in self.blind.items()
            for ordinal in range(1, spec.unit_count + 1)
        }
        return frozenset(self.conformance) | blind_ids

    def capture_identities(self) -> dict[str, tuple[str, str, int]]:
        result = {
            unit_id: (unit.source_id, unit.source_sha256, unit.ordinal)
            for unit_id, unit in self.conformance.items()
        }
        result.update(
            {
                f"{source_id}-unit-{ordinal}": (
                    source_id,
                    item.source_sha256,
                    ordinal,
                )
                for source_id, item in self.blind.items()
                for ordinal in range(1, item.unit_count + 1)
            }
        )
        return result


class RoundedSummary(TypedDict):
    score: float
    visual: float
    content: float
    layout: float
    minimum: float


@dataclass(frozen=True, slots=True)
class ScoreSummary:
    count: int
    visual: Decimal
    content: Decimal
    layout: Decimal
    score: Decimal
    minimum: Decimal

    def rounded(self) -> RoundedSummary:
        return {
            "score": rounded_float(self.score),
            "visual": rounded_float(self.visual),
            "content": rounded_float(self.content),
            "layout": rounded_float(self.layout),
            "minimum": rounded_float(self.minimum),
        }


def rounded_float(value: Decimal) -> float:
    return float(retained_decimal(value))


def retained_decimal(value: Decimal) -> Decimal:
    return value.quantize(SIX_PLACES, rounding=ROUND_HALF_EVEN)


@dataclass(frozen=True, slots=True)
class DeterminismSummary:
    runs: int
    html_hashes_equal: bool
    inventory_hashes_equal: bool
    png_hashes_equal: bool


@dataclass(frozen=True, slots=True)
class QualitySummary:
    tests_passed: bool
    builds_passed: bool
    diagnostics_passed: bool
    contract_checks_passed: bool

    def all_passed(self) -> bool:
        return all(
            [
                self.tests_passed,
                self.builds_passed,
                self.diagnostics_passed,
                self.contract_checks_passed,
            ]
        )


@dataclass(frozen=True, slots=True)
class MetricsSummary:
    status: MetricStatus
    document_format: DocumentFormat
    conformance: ScoreSummary
    conformance_strata: dict[str, Decimal]
    conformance_critical_defects: int
    blind: ScoreSummary
    blind_accepted_files: int
    blind_critical_defects: int
    security_cases: int
    security_passed: int
    determinism: DeterminismSummary
    reviewer_count: int
    review_all_passed: bool
    quality: QualitySummary
    performance_within_limits: bool


@dataclass(frozen=True, slots=True)
class ConformanceMetricResult:
    summary: ScoreSummary
    strata: dict[str, Decimal]
    critical_defects: int
    artifact_paths: frozenset[str]


@dataclass(frozen=True, slots=True)
class BlindMetricResult:
    summary: ScoreSummary
    accepted_files: int
    critical_defects: int
    artifact_paths: frozenset[str]


@dataclass(frozen=True, slots=True)
class HardGateSummary:
    security_cases: int
    security_passed: int
    determinism: DeterminismSummary
    reviewer_count: int
    review_all_passed: bool
    quality: QualitySummary
    performance_within_limits: bool
    artifact_paths: frozenset[str]
