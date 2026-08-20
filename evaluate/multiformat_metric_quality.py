from __future__ import annotations

from pathlib import Path

from evaluate.multiformat_corpus_items import require_keys
from evaluate.multiformat_metric_compute import resolve_artifact_binding
from evaluate.multiformat_metric_types import (
    MetricError,
    QualitySummary,
)
from evaluate.multiformat_schema import (
    JsonValue,
    boolean_value,
    integer_value,
    object_value,
    sha256_value,
    string_value,
)
from evaluate.multiformat_strict_json import StrictJsonError, read_strict_object


def compute_quality(
    values: dict[str, JsonValue],
    performance: dict[str, JsonValue],
    evidence_root: Path,
    evaluator_hash: str,
    corpus_hash: str,
    project_revision: str,
) -> tuple[QualitySummary, bool, set[Path]]:
    quality_fields = {
        "tests",
        "builds",
        "diagnostics",
        "contract_checks",
    }
    require_keys(values, quality_fields, "quality")
    paths = {
        field: resolve_artifact_binding(
            object_value(values, field),
            evidence_root,
            f"quality.{field}",
        )
        for field in quality_fields
    }
    if len(set(paths.values())) != len(paths):
        raise MetricError("artifact.path", "quality evidence is reused")
    require_keys(performance, {"evidence"}, "performance")
    performance_path = resolve_artifact_binding(
        object_value(performance, "evidence"),
        evidence_root,
        "performance.evidence",
    )
    if performance_path in paths.values():
        raise MetricError("artifact.path", performance_path.as_posix())
    summary = QualitySummary(
        tests_passed=_quality_result(
            paths["tests"],
            "tests",
            evaluator_hash,
            corpus_hash,
            project_revision,
        ),
        builds_passed=_quality_result(
            paths["builds"],
            "builds",
            evaluator_hash,
            corpus_hash,
            project_revision,
        ),
        diagnostics_passed=_quality_result(
            paths["diagnostics"],
            "diagnostics",
            evaluator_hash,
            corpus_hash,
            project_revision,
        ),
        contract_checks_passed=_quality_result(
            paths["contract_checks"],
            "contract_checks",
            evaluator_hash,
            corpus_hash,
            project_revision,
        ),
    )
    return (
        summary,
        _performance_result(
            performance_path,
            evaluator_hash,
            corpus_hash,
            project_revision,
        ),
        {*paths.values(), performance_path},
    )


def _quality_result(
    path: Path,
    command_id: str,
    evaluator_hash: str,
    corpus_hash: str,
    project_revision: str,
) -> bool:
    try:
        values = read_strict_object(path)
        require_keys(
            values,
            {
                "schema_version",
                "status",
                "command_id",
                "exit_code",
                "project_revision",
                "evaluator_manifest_sha256",
                "corpus_manifest_sha256",
            },
            "quality.result",
        )
        return all(
            [
                integer_value(values, "schema_version") == 1,
                string_value(values, "status") == "PASS",
                string_value(values, "command_id") == command_id,
                integer_value(values, "exit_code") == 0,
                string_value(values, "project_revision") == project_revision,
                sha256_value(values, "evaluator_manifest_sha256") == evaluator_hash,
                sha256_value(values, "corpus_manifest_sha256") == corpus_hash,
            ]
        )
    except (StrictJsonError, TypeError, ValueError) as error:
        raise MetricError("quality", path.as_posix()) from error


def _performance_result(
    path: Path,
    evaluator_hash: str,
    corpus_hash: str,
    project_revision: str,
) -> bool:
    try:
        values = read_strict_object(path)
        require_keys(
            values,
            {
                "schema_version",
                "status",
                "within_limits",
                "project_revision",
                "evaluator_manifest_sha256",
                "corpus_manifest_sha256",
            },
            "performance.result",
        )
        return all(
            [
                integer_value(values, "schema_version") == 1,
                string_value(values, "status") == "PASS",
                boolean_value(values, "within_limits"),
                string_value(values, "project_revision") == project_revision,
                sha256_value(values, "evaluator_manifest_sha256") == evaluator_hash,
                sha256_value(values, "corpus_manifest_sha256") == corpus_hash,
            ]
        )
    except (StrictJsonError, TypeError, ValueError) as error:
        raise MetricError("performance", path.as_posix()) from error
