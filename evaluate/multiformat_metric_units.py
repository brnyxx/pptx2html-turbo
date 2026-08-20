from __future__ import annotations

from collections import defaultdict
from decimal import Decimal
from pathlib import Path

from evaluate.multiformat_corpus_items import object_list, require_keys
from evaluate.multiformat_corpus_types import CorpusError
from evaluate.multiformat_metric_aggregate import (
    aggregate_files,
    aggregate_units,
)
from evaluate.multiformat_metric_compute import compute_unit
from evaluate.multiformat_metric_types import (
    BlindMetricResult,
    ConformanceMetricResult,
    CorpusMetricSpec,
    MetricError,
    ScoreSummary,
    UnitScore,
)
from evaluate.multiformat_schema import (
    JsonValue,
    boolean_value,
    integer_value,
    sha256_value,
    string_value,
)


def compute_conformance(
    values: dict[str, JsonValue],
    spec: CorpusMetricSpec,
    evidence_root: Path,
) -> ConformanceMetricResult:
    try:
        require_keys(values, {"units"}, "conformance")
        records = object_list(values, "units", "conformance.units")
        scores: dict[str, UnitScore] = {}
        strata: dict[str, list[UnitScore]] = defaultdict(list)
        artifacts: set[str] = set()
        critical_defects = 0
        for record in records:
            require_keys(
                record,
                {
                    "source_id",
                    "source_sha256",
                    "unit_id",
                    "ordinal",
                    "critical_defect",
                    "artifacts",
                },
                "conformance.unit",
            )
            unit_id = string_value(record, "unit_id")
            expected = spec.conformance.get(unit_id)
            if expected is None or unit_id in scores:
                raise MetricError("conformance.unit_set", unit_id)
            if (
                string_value(record, "source_id") != expected.source_id
                or sha256_value(record, "source_sha256") != expected.source_sha256
                or integer_value(record, "ordinal") != expected.ordinal
            ):
                raise MetricError("conformance.unit_identity", unit_id)
            computed = compute_unit(
                record,
                unit_id,
                expected.applicable_metrics,
                expected.background,
                spec.document_format,
                evidence_root,
            )
            _add_artifacts(artifacts, computed.artifacts.paths())
            scores[unit_id] = computed.score
            strata[expected.stratum].append(computed.score)
            critical_defects += int(boolean_value(record, "critical_defect"))
        if set(scores) != set(spec.conformance):
            raise MetricError("conformance.unit_set", "missing or extra unit")
        summary = aggregate_units(list(scores.values()))
        stratum_scores = {
            name: aggregate_units(units).score for name, units in strata.items()
        }
        return ConformanceMetricResult(
            summary,
            stratum_scores,
            critical_defects,
            frozenset(artifacts),
        )
    except MetricError:
        raise
    except (CorpusError, TypeError, ValueError) as error:
        raise MetricError("metrics.conformance", "invalid evidence") from error


def compute_blind(
    values: dict[str, JsonValue],
    spec: CorpusMetricSpec,
    evidence_root: Path,
    minimum_file_score: Decimal,
) -> BlindMetricResult:
    try:
        require_keys(values, {"files"}, "blind")
        records = object_list(values, "files", "blind.files")
        file_scores: dict[str, ScoreSummary] = {}
        unit_groups: list[list[UnitScore]] = []
        artifacts: set[str] = set()
        critical_defects = 0
        accepted_files = 0
        for record in records:
            require_keys(
                record,
                {
                    "source_id",
                    "source_sha256",
                    "critical_defect",
                    "units",
                },
                "blind.file",
            )
            source_id = string_value(record, "source_id")
            expected = spec.blind.get(source_id)
            if expected is None or source_id in file_scores:
                raise MetricError("blind.file_set", source_id)
            if sha256_value(record, "source_sha256") != expected.source_sha256:
                raise MetricError("blind.file_identity", source_id)
            unit_records = object_list(record, "units", "blind.units")
            scores = _blind_units(
                unit_records,
                expected.unit_count,
                expected.applicable_metrics,
                expected.background,
                source_id,
                spec,
                evidence_root,
                artifacts,
            )
            summary = aggregate_units(scores)
            file_scores[source_id] = summary
            unit_groups.append(scores)
            file_critical = boolean_value(record, "critical_defect") or any(
                boolean_value(unit, "critical_defect") for unit in unit_records
            )
            critical_defects += int(file_critical)
            accepted_files += int(
                not file_critical and summary.score >= minimum_file_score
            )
        if set(file_scores) != set(spec.blind):
            raise MetricError("blind.file_set", "missing or extra file")
        return BlindMetricResult(
            aggregate_files(unit_groups),
            accepted_files,
            critical_defects,
            frozenset(artifacts),
        )
    except MetricError:
        raise
    except (CorpusError, TypeError, ValueError) as error:
        raise MetricError("metrics.blind", "invalid evidence") from error


def _blind_units(
    records: list[dict[str, JsonValue]],
    expected_count: int,
    applicable_metrics: frozenset[str],
    background: str,
    source_id: str,
    spec: CorpusMetricSpec,
    evidence_root: Path,
    artifacts: set[str],
) -> list[UnitScore]:
    scores: list[UnitScore] = []
    for ordinal, record in enumerate(records, start=1):
        require_keys(
            record,
            {"unit_id", "ordinal", "critical_defect", "artifacts"},
            "blind.unit",
        )
        expected_id = f"{source_id}-unit-{ordinal}"
        if (
            string_value(record, "unit_id") != expected_id
            or integer_value(record, "ordinal") != ordinal
        ):
            raise MetricError("blind.unit_set", source_id)
        computed = compute_unit(
            record,
            expected_id,
            applicable_metrics,
            background,
            spec.document_format,
            evidence_root,
        )
        _add_artifacts(artifacts, computed.artifacts.paths())
        scores.append(computed.score)
    if len(scores) != expected_count:
        raise MetricError("blind.unit_set", source_id)
    return scores


def _add_artifacts(values: set[str], paths: frozenset[Path]) -> None:
    normalized = {path.as_posix() for path in paths}
    if values & normalized:
        raise MetricError("artifact.path", "artifact reused across units")
    values.update(normalized)
