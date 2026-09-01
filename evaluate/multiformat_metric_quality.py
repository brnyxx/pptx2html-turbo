from __future__ import annotations

from pathlib import Path

from evaluate.multiformat_command_evidence import (
    CommandIdentity,
    CommandPlan,
    command_value,
)
from evaluate.multiformat_corpus_items import require_keys
from evaluate.multiformat_metric_compute import resolve_artifact_binding
from evaluate.multiformat_metric_types import MetricError, QualitySummary
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
    command_plan: CommandPlan,
) -> tuple[QualitySummary, bool, set[Path]]:
    fields = {"tests", "builds", "diagnostics", "contract_checks"}
    require_keys(values, fields, "quality")
    paths = {
        field: resolve_artifact_binding(
            object_value(values, field), evidence_root, f"quality.{field}"
        )
        for field in fields
    }
    if len(set(paths.values())) != len(paths):
        raise MetricError("artifact.path", "quality evidence is reused")
    require_keys(performance, {"evidence"}, "performance")
    performance_path = resolve_artifact_binding(
        object_value(performance, "evidence"), evidence_root, "performance.evidence"
    )
    if performance_path in paths.values():
        raise MetricError("artifact.path", performance_path.as_posix())
    results = {
        field: _quality_result(
            paths[field],
            field,
            evaluator_hash,
            corpus_hash,
            project_revision,
            command_plan.quality[field],
            command_plan.sha256,
        )
        for field in fields
    }
    summary = QualitySummary(
        results["tests"],
        results["builds"],
        results["diagnostics"],
        results["contract_checks"],
    )
    performance_result = _performance_result(
        performance_path,
        evaluator_hash,
        corpus_hash,
        project_revision,
        command_plan.performance,
        command_plan.sha256,
    )
    return summary, performance_result, {*paths.values(), performance_path}


def _quality_result(
    path: Path,
    command_id: str,
    evaluator_hash: str,
    corpus_hash: str,
    project_revision: str,
    command: CommandIdentity,
    plan_hash: str,
) -> bool:
    try:
        values = read_strict_object(path)
        require_keys(
            values,
            {
                "schema_version",
                "status",
                "command_id",
                "command_plan_sha256",
                "command",
                "exit_code",
                "project_revision",
                "evaluator_manifest_sha256",
                "corpus_manifest_sha256",
            },
            "quality.result",
        )
        return all(
            [
                integer_value(values, "schema_version") == 2,
                string_value(values, "status") == "PASS",
                string_value(values, "command_id") == command_id,
                sha256_value(values, "command_plan_sha256") == plan_hash,
                object_value(values, "command") == command_value(command),
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
    command: CommandIdentity,
    plan_hash: str,
) -> bool:
    try:
        values = read_strict_object(path)
        require_keys(
            values,
            {
                "schema_version",
                "status",
                "within_limits",
                "command_plan_sha256",
                "command",
                "project_revision",
                "evaluator_manifest_sha256",
                "corpus_manifest_sha256",
            },
            "performance.result",
        )
        return all(
            [
                integer_value(values, "schema_version") == 2,
                string_value(values, "status") == "PASS",
                boolean_value(values, "within_limits"),
                sha256_value(values, "command_plan_sha256") == plan_hash,
                object_value(values, "command") == command_value(command),
                string_value(values, "project_revision") == project_revision,
                sha256_value(values, "evaluator_manifest_sha256") == evaluator_hash,
                sha256_value(values, "corpus_manifest_sha256") == corpus_hash,
            ]
        )
    except (StrictJsonError, TypeError, ValueError) as error:
        raise MetricError("performance", path.as_posix()) from error
