from __future__ import annotations

from pathlib import Path

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
) -> None:
    producer = string_value(values, "producer")
    runtime_hash = sha256_value(values, "runtime_sha256")
    if producer != _expected_producer(role, spec.document_format.value):
        raise MetricError("metrics.binding.capture", role)
    _validate_rendering(
        object_value(values, "rendering"),
        spec.document_format.value,
    )
    upstream_path = resolve_artifact_binding(
        object_value(values, "upstream_manifest"),
        evidence_root,
        "capture.upstream_manifest",
    )
    _validate_upstream(
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
        evidence_root,
        values.get("units"),
        values.get("files"),
    )


def _validate_rendering(
    values: dict[str, JsonValue],
    document_format: str,
) -> None:
    require_keys(values, {"dpi", "width", "height"}, "capture.rendering")
    if document_format in {"ppt", "pptx"}:
        valid = (
            values.get("dpi") is None
            and integer_value(values, "width") == 960
            and integer_value(values, "height") == 540
        )
    else:
        valid = (
            integer_value(values, "dpi") == 144
            and values.get("width") is None
            and values.get("height") is None
        )
    if not valid:
        raise MetricError("artifact.dimension", document_format)


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
    evidence_root: Path,
    outer_units: JsonValue,
    outer_files: JsonValue,
) -> None:
    values = read_strict_object(path)
    require_keys(
        values,
        {
            "schema_version",
            "status",
            "role",
            "format",
            "producer",
            "runtime_sha256",
            "project_revision",
            "contract_sha256",
            "corpus_manifest_sha256",
            "evaluator_manifest_sha256",
            "oracle_lock_sha256",
            "units",
            "files",
            "execution_log",
        },
        "capture.upstream",
    )
    revision = string_value(values, "project_revision")
    if (
        integer_value(values, "schema_version") != 1
        or string_value(values, "status") != "READY"
        or string_value(values, "role") != role
        or string_value(values, "format") != spec.document_format.value
        or string_value(values, "producer") != producer
        or sha256_value(values, "runtime_sha256") != runtime_hash
        or revision != project_revision
        or sha256_value(values, "contract_sha256") != contract_hash
        or sha256_value(values, "corpus_manifest_sha256") != corpus_hash
        or sha256_value(values, "evaluator_manifest_sha256") != evaluator_hash
        or sha256_value(values, "oracle_lock_sha256") != oracle_hash
        or values.get("units") != outer_units
        or values.get("files") != outer_files
    ):
        raise MetricError("metrics.binding.capture", f"{role} upstream")
    log_path = resolve_artifact_binding(
        object_value(values, "execution_log"),
        evidence_root,
        "capture.execution_log",
    )
    log_values = read_strict_object(log_path)
    require_keys(log_values, {"schema_version", "status", "role"}, "capture.log")
    if (
        integer_value(log_values, "schema_version") != 1
        or string_value(log_values, "status") != "PASS"
        or string_value(log_values, "role") != role
    ):
        raise MetricError("metrics.binding.capture", f"{role} execution")


def _expected_producer(role: str, document_format: str) -> str:
    if role == "candidate":
        return "document2html-candidate"
    if document_format == "pdf":
        return "locked-pdf-renderer"
    return "windows-office-native"
