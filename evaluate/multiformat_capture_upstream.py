from __future__ import annotations

from pathlib import Path

from evaluate.multiformat_capture_contract import capture_counts
from evaluate.multiformat_corpus_items import require_keys
from evaluate.multiformat_metric_compute import resolve_artifact_binding
from evaluate.multiformat_metric_types import CorpusMetricSpec, MetricError
from evaluate.multiformat_schema import (
    JsonValue,
    integer_value,
    object_value,
    sha256_value,
    string_value,
)
from evaluate.multiformat_strict_json import read_strict_object


def validate_capture_upstream(
    path: Path,
    role: str,
    spec: CorpusMetricSpec,
    contract_hash: str,
    corpus_hash: str,
    evaluator_hash: str,
    oracle_hash: str,
    project_revision: str,
    producer: str,
    runtime_hash: str,
    runtime_binding: dict[str, JsonValue],
    evidence_root: Path,
    outer_units: JsonValue,
    outer_files: JsonValue,
    outer_determinism: JsonValue,
    outer_receipt: JsonValue,
    outer_office_batch: JsonValue,
) -> Path:
    values = read_strict_object(path)
    required_fields = {
        "schema_version",
        "status",
        "role",
        "format",
        "producer",
        "runtime_sha256",
        "runtime_identity",
        "project_revision",
        "contract_sha256",
        "corpus_manifest_sha256",
        "evaluator_manifest_sha256",
        "oracle_lock_sha256",
        "units",
        "files",
        "execution_log",
    }
    if role == "candidate":
        required_fields.add("determinism_manifest")
    if outer_receipt is not None:
        required_fields.add("execution_receipt")
    if outer_office_batch is not None:
        required_fields.add("office_batch_manifest")
    require_keys(values, required_fields, "capture.upstream")
    if (
        integer_value(values, "schema_version") != 1
        or string_value(values, "status") != "READY"
        or string_value(values, "role") != role
        or string_value(values, "format") != spec.document_format.value
        or string_value(values, "producer") != producer
        or sha256_value(values, "runtime_sha256") != runtime_hash
        or object_value(values, "runtime_identity") != runtime_binding
        or string_value(values, "project_revision") != project_revision
        or sha256_value(values, "contract_sha256") != contract_hash
        or sha256_value(values, "corpus_manifest_sha256") != corpus_hash
        or sha256_value(values, "evaluator_manifest_sha256") != evaluator_hash
        or sha256_value(values, "oracle_lock_sha256") != oracle_hash
        or values.get("units") != outer_units
        or values.get("files") != outer_files
        or values.get("determinism_manifest") != outer_determinism
        or values.get("execution_receipt") != outer_receipt
        or values.get("office_batch_manifest") != outer_office_batch
    ):
        raise MetricError("metrics.binding.capture", f"{role} upstream")
    log_path = resolve_artifact_binding(
        object_value(values, "execution_log"),
        evidence_root,
        "capture.execution_log",
    )
    _validate_execution_log(
        read_strict_object(log_path),
        role,
        project_revision,
        evaluator_hash,
        corpus_hash,
        outer_units,
    )
    return log_path


def _validate_execution_log(
    values: dict[str, JsonValue],
    role: str,
    project_revision: str,
    evaluator_hash: str,
    corpus_hash: str,
    outer_units: JsonValue,
) -> None:
    require_keys(
        values,
        {
            "schema_version",
            "status",
            "role",
            "project_revision",
            "evaluator_manifest_sha256",
            "corpus_manifest_sha256",
            "network_isolation",
            "source_count",
            "unit_count",
            "external_requests",
            "determinism_runs",
        },
        "capture.log",
    )
    source_count, unit_count = capture_counts(outer_units)
    if (
        integer_value(values, "schema_version") != 1
        or string_value(values, "status") != "PASS"
        or string_value(values, "role") != role
        or string_value(values, "project_revision") != project_revision
        or sha256_value(values, "evaluator_manifest_sha256") != evaluator_hash
        or sha256_value(values, "corpus_manifest_sha256") != corpus_hash
        or string_value(values, "network_isolation") != "disabled"
        or integer_value(values, "source_count") != source_count
        or integer_value(values, "unit_count") != unit_count
        or values.get("external_requests") != []
        or integer_value(values, "determinism_runs")
        != (2 if role == "candidate" else 1)
    ):
        raise MetricError("metrics.binding.capture", f"{role} execution")
