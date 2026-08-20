from __future__ import annotations

from pathlib import Path

from evaluate.multiformat_corpus_items import object_list, require_keys
from evaluate.multiformat_corpus_types import SecurityOutcome
from evaluate.multiformat_metric_compute import resolve_artifact_binding
from evaluate.multiformat_metric_types import (
    CorpusMetricSpec,
    MetricError,
)
from evaluate.multiformat_schema import (
    JsonValue,
    boolean_value,
    integer_value,
    object_value,
    sha256_value,
    string_list,
    string_value,
)
from evaluate.multiformat_strict_json import read_strict_object


def compute_security(
    values: dict[str, JsonValue],
    spec: CorpusMetricSpec,
    evidence_root: Path,
    evaluator_hash: str,
    corpus_hash: str,
    project_revision: str,
) -> tuple[int, int, set[Path]]:
    require_keys(values, {"cases"}, "security")
    records = object_list(values, "cases", "security.cases")
    seen: set[str] = set()
    executions: set[Path] = set()
    passed = 0
    for record in records:
        require_keys(
            record,
            {"source_id", "execution"},
            "security.case",
        )
        source_id = string_value(record, "source_id")
        expected = spec.security.get(source_id)
        if expected is None or source_id in seen:
            raise MetricError("security.case_set", source_id)
        seen.add(source_id)
        execution = resolve_artifact_binding(
            object_value(record, "execution"),
            evidence_root,
            "security.execution",
        )
        if execution in executions:
            raise MetricError("artifact.path", source_id)
        executions.add(execution)
        result = read_strict_object(execution)
        require_keys(
            result,
            {
                "schema_version",
                "status",
                "source_id",
                "source_sha256",
                "case_family",
                "expected_outcome",
                "observed_outcome",
                "typed_error",
                "network_isolation",
                "external_fetches",
                "active_content_executed",
                "within_limits",
                "project_revision",
                "evaluator_manifest_sha256",
                "corpus_manifest_sha256",
            },
            "security.execution",
        )
        expected_outcome = SecurityOutcome(string_value(result, "expected_outcome"))
        observed_outcome = SecurityOutcome(string_value(result, "observed_outcome"))
        if (
            integer_value(result, "schema_version") != 1
            or string_value(result, "status") != "PASS"
            or string_value(result, "source_id") != source_id
            or sha256_value(result, "source_sha256") != expected.source_sha256
            or string_value(result, "case_family") != expected.case_family
            or expected_outcome is not expected.expected_outcome
            or string_value(result, "project_revision") != project_revision
            or sha256_value(result, "evaluator_manifest_sha256") != evaluator_hash
            or sha256_value(result, "corpus_manifest_sha256") != corpus_hash
        ):
            raise MetricError("security.case_set", source_id)
        typed_error = result.get("typed_error")
        typed_error_valid = (
            isinstance(typed_error, str) and bool(typed_error)
            if expected_outcome is SecurityOutcome.REJECT
            else typed_error is None
        )
        case_passed = all(
            [
                observed_outcome is expected_outcome,
                typed_error_valid,
                string_value(result, "network_isolation") == "disabled",
                not string_list(result, "external_fetches"),
                not boolean_value(result, "active_content_executed"),
                boolean_value(result, "within_limits"),
            ]
        )
        passed += int(case_passed)
    if seen != set(spec.security):
        raise MetricError("security.case_set", "missing or extra case")
    return len(records), passed, executions
