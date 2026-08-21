from __future__ import annotations

from pathlib import Path

from evaluate.multiformat_capture_contract import (
    capture_counts,
    expected_capture_producer,
)
from evaluate.multiformat_capture_receipt import validate_execution_receipt
from evaluate.multiformat_capture_rendering import validate_capture_rendering
from evaluate.multiformat_capture_runtime import validate_capture_runtime
from evaluate.multiformat_corpus_items import require_keys
from evaluate.multiformat_metric_compute import resolve_artifact_binding
from evaluate.multiformat_metric_types import CorpusMetricSpec, MetricError
from evaluate.multiformat_office_oracle_provenance import (
    office_oracle_bindings,
    validate_office_oracle_provenance,
)
from evaluate.multiformat_schema import (
    JsonValue,
    integer_value,
    object_value,
    sha256_value,
    string_value,
)
from evaluate.multiformat_strict_json import read_strict_object


def validate_capture_provenance(
    values: dict[str, JsonValue],
    role: str,
    spec: CorpusMetricSpec,
    contract_hash: str,
    corpus_hash: str,
    evaluator_hash: str,
    oracle_hash: str,
    project_revision: str,
    evidence_root: Path,
    oracle_lock_path: Path | None,
) -> None:
    producer = string_value(values, "producer")
    runtime_hash = sha256_value(values, "runtime_sha256")
    if producer != expected_capture_producer(role, spec.document_format.value):
        raise MetricError("metrics.binding.capture", role)
    validate_capture_rendering(
        object_value(values, "rendering"),
        spec.document_format.value,
    )
    runtime_binding = object_value(values, "runtime_identity")
    runtime_path = resolve_artifact_binding(
        runtime_binding,
        evidence_root,
        "capture.runtime_identity",
    )
    if sha256_value(runtime_binding, "sha256") != runtime_hash:
        raise MetricError("metrics.binding.capture", f"{role} runtime")
    validate_capture_runtime(
        runtime_path,
        role,
        producer,
        project_revision,
        evidence_root,
        oracle_lock_path,
        contract_hash,
        corpus_hash,
        evaluator_hash,
        oracle_hash,
    )
    upstream_path = resolve_artifact_binding(
        object_value(values, "upstream_manifest"),
        evidence_root,
        "capture.upstream_manifest",
    )
    determinism_binding = (
        object_value(values, "determinism_manifest") if role == "candidate" else None
    )
    receipt_binding = (
        object_value(values, "execution_receipt") if role == "candidate" else None
    )
    office_receipt, office_batch_binding = office_oracle_bindings(
        values,
        role,
        oracle_lock_path,
    )
    receipt_binding = receipt_binding or office_receipt
    execution_path = _validate_upstream(
        upstream_path,
        role,
        spec,
        contract_hash,
        corpus_hash,
        evaluator_hash,
        oracle_hash,
        project_revision,
        producer,
        runtime_hash,
        runtime_binding,
        evidence_root,
        values.get("units"),
        values.get("files"),
        determinism_binding,
        receipt_binding,
        office_batch_binding,
    )
    if role == "candidate":
        if determinism_binding is None or receipt_binding is None:
            raise MetricError("metrics.binding.capture", "candidate receipt")
        determinism_path = resolve_artifact_binding(
            determinism_binding,
            evidence_root,
            "capture.determinism_manifest",
        )
        receipt_path = resolve_artifact_binding(
            receipt_binding,
            evidence_root,
            "capture.execution_receipt",
        )
        if oracle_lock_path is not None:
            validate_execution_receipt(
                receipt_path,
                determinism_path,
                runtime_path,
                execution_path,
                evidence_root,
                oracle_lock_path,
                project_revision=project_revision,
                contract_sha256=contract_hash,
                corpus_sha256=corpus_hash,
                evaluator_sha256=evaluator_hash,
                oracle_lock_sha256=oracle_hash,
            )
    elif oracle_lock_path is not None:
        if receipt_binding is None or office_batch_binding is None:
            raise MetricError("metrics.binding.capture", "office oracle receipt")
        validate_office_oracle_provenance(
            receipt_binding=receipt_binding,
            batch_binding=office_batch_binding,
            units=values.get("units"),
            runtime_path=runtime_path,
            execution_path=execution_path,
            producer=producer,
            project_revision=project_revision,
            contract_hash=contract_hash,
            corpus_hash=corpus_hash,
            evaluator_hash=evaluator_hash,
            oracle_hash=oracle_hash,
            evidence_root=evidence_root,
            oracle_lock_path=oracle_lock_path,
        )


def _validate_upstream(
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
        required_fields |= {"determinism_manifest", "execution_receipt"}
    elif outer_office_batch is not None:
        required_fields |= {"office_batch_manifest", "execution_receipt"}
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
    log_values = read_strict_object(log_path)
    require_keys(
        log_values,
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
        integer_value(log_values, "schema_version") != 1
        or string_value(log_values, "status") != "PASS"
        or string_value(log_values, "role") != role
        or string_value(log_values, "project_revision") != project_revision
        or sha256_value(log_values, "evaluator_manifest_sha256") != evaluator_hash
        or sha256_value(log_values, "corpus_manifest_sha256") != corpus_hash
        or string_value(log_values, "network_isolation") != "disabled"
        or integer_value(log_values, "source_count") != source_count
        or integer_value(log_values, "unit_count") != unit_count
        or log_values.get("external_requests") != []
        or integer_value(log_values, "determinism_runs")
        != (2 if role == "candidate" else 1)
    ):
        raise MetricError("metrics.binding.capture", f"{role} execution")
    return log_path
