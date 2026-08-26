from __future__ import annotations

from pathlib import Path

from evaluate import multiformat_portable_receipt as portable_receipt
from evaluate.multiformat_capture_contract import expected_capture_producer
from evaluate.multiformat_capture_profile import CaptureProfileContext
from evaluate.multiformat_capture_receipt import validate_execution_receipt
from evaluate.multiformat_capture_rendering import validate_capture_rendering
from evaluate.multiformat_capture_runtime import validate_capture_runtime
from evaluate.multiformat_capture_upstream import validate_capture_upstream
from evaluate.multiformat_metric_compute import resolve_artifact_binding
from evaluate.multiformat_metric_types import CorpusMetricSpec, MetricError
from evaluate.multiformat_portable_capture import validate_portable_provenance
from evaluate.multiformat_office_oracle_provenance import (
    office_oracle_bindings,
    validate_office_oracle_provenance,
)
from evaluate.multiformat_schema import (
    JsonValue,
    object_value,
    sha256_value,
    string_value,
)

validate_portable_capture_provenance = portable_receipt.verify_portable_receipt


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
    profile: CaptureProfileContext,
) -> None:
    producer = string_value(values, "producer")
    runtime_hash = sha256_value(values, "runtime_sha256")
    if producer != expected_capture_producer(
        role,
        spec.document_format.value,
        profile.profile,
    ):
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
        profile.lock_path,
        contract_hash,
        corpus_hash,
        evaluator_hash,
        oracle_hash,
        profile,
    )
    upstream_path = resolve_artifact_binding(
        object_value(values, "upstream_manifest"),
        evidence_root,
        "capture.upstream_manifest",
    )
    determinism_binding = (
        object_value(values, "determinism_manifest") if role == "candidate" else None
    )
    receipt_binding = _receipt_binding(values, role, profile)
    office_batch_binding = _office_batch_binding(values, role, profile)
    execution_path = validate_capture_upstream(
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
    if receipt_binding is None:
        return
    receipt_path = resolve_artifact_binding(
        receipt_binding,
        evidence_root,
        "capture.execution_receipt",
    )
    if profile.portable_trust is not None:
        validate_portable_provenance(
            receipt_path=receipt_path,
            values=values,
            runtime_path=runtime_path,
            execution_path=execution_path,
            evidence_root=evidence_root,
            trust=profile.portable_trust,
            project_revision=project_revision,
            contract_hash=contract_hash,
            corpus_hash=corpus_hash,
            evaluator_hash=evaluator_hash,
            oracle_hash=oracle_hash,
        )
    elif role == "candidate" and profile.lock_path is not None:
        if determinism_binding is None:
            raise MetricError("metrics.binding.capture", "candidate receipt")
        validate_execution_receipt(
            receipt_path,
            resolve_artifact_binding(
                determinism_binding,
                evidence_root,
                "capture.determinism_manifest",
            ),
            runtime_path,
            execution_path,
            evidence_root,
            profile.lock_path,
            project_revision=project_revision,
            contract_sha256=contract_hash,
            corpus_sha256=corpus_hash,
            evaluator_sha256=evaluator_hash,
            oracle_lock_sha256=oracle_hash,
        )
    elif profile.lock_path is not None and office_batch_binding is not None:
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
            oracle_lock_path=profile.lock_path,
        )


def _receipt_binding(
    values: dict[str, JsonValue],
    role: str,
    profile: CaptureProfileContext,
) -> dict[str, JsonValue] | None:
    if role == "candidate" or profile.is_portable:
        return object_value(values, "execution_receipt")
    receipt, _ = office_oracle_bindings(values, role, profile.lock_path)
    return receipt


def _office_batch_binding(
    values: dict[str, JsonValue],
    role: str,
    profile: CaptureProfileContext,
) -> dict[str, JsonValue] | None:
    if profile.is_portable:
        return None
    _, batch = office_oracle_bindings(values, role, profile.lock_path)
    return batch
