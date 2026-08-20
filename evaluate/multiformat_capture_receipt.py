from __future__ import annotations

import hashlib
from pathlib import Path

from evaluate.multiformat_candidate_attestation import (
    CandidateAttestationError,
    canonical_payload,
    verify_signed_payload,
)
from evaluate.multiformat_corpus_items import object_list
from evaluate.multiformat_metric_compute import resolve_artifact_binding
from evaluate.multiformat_metric_types import MetricError
from evaluate.multiformat_schema import (
    JsonValue,
    object_value,
    sha256_file,
    string_value,
)
from evaluate.multiformat_strict_json import read_strict_object


def validate_execution_receipt(
    receipt_path: Path,
    determinism_path: Path,
    runtime_path: Path,
    execution_path: Path,
    evidence_root: Path,
    oracle_lock_path: Path,
    *,
    project_revision: str,
    contract_sha256: str,
    corpus_sha256: str,
    evaluator_sha256: str,
    oracle_lock_sha256: str,
) -> None:
    runtime = read_strict_object(runtime_path)
    tools = object_value(runtime, "tools")
    runtime_artifacts = object_value(runtime, "artifacts")
    artifacts = receipt_artifacts_from_manifests(
        determinism_path,
        runtime_artifacts,
        evidence_root,
    )
    lock = read_strict_object(oracle_lock_path)
    payload: dict[str, JsonValue] = {
        "schema_version": 1,
        "status": "PASS",
        "verifier_id": string_value(
            object_value(lock, "sandbox_verifier"),
            "verifier_id",
        ),
        "run_nonce": string_value(tools, "run_nonce"),
        "project_revision": project_revision,
        "contract_sha256": contract_sha256,
        "corpus_manifest_sha256": corpus_sha256,
        "evaluator_manifest_sha256": evaluator_sha256,
        "oracle_lock_sha256": oracle_lock_sha256,
        "runtime_identity_sha256": sha256_file(runtime_path),
        "execution_log_sha256": sha256_file(execution_path),
        "determinism_sha256": sha256_file(determinism_path),
        "artifact_root_sha256": hashlib.sha256(
            canonical_payload({"artifacts": artifacts})
        ).hexdigest(),
        "artifacts": artifacts,
    }
    paths = {
        name: resolve_artifact_binding(
            object_value(runtime_artifacts, name),
            evidence_root,
            f"receipt.{name}",
        )
        for name in ["sandbox_public_key", "openssl_binary"]
    }
    try:
        verify_signed_payload(
            receipt_path,
            paths["sandbox_public_key"],
            paths["openssl_binary"],
            oracle_lock_path,
            payload,
        )
    except CandidateAttestationError as error:
        raise MetricError("metrics.binding.capture", "execution receipt") from error


def receipt_artifacts_from_manifests(
    determinism_path: Path,
    runtime_artifacts: dict[str, JsonValue],
    evidence_root: Path,
) -> list[dict[str, JsonValue]]:
    artifact_bindings = [
        object_value(runtime_artifacts, name) for name in runtime_artifacts
    ]
    determinism = read_strict_object(determinism_path)
    for run in object_list(determinism, "runs", "receipt.runs"):
        for file_record in object_list(run, "files", "receipt.files"):
            artifact_bindings.append(object_value(file_record, "html"))
            inventory_binding = object_value(file_record, "inventory")
            artifact_bindings.append(inventory_binding)
            inventory_path = resolve_artifact_binding(
                inventory_binding,
                evidence_root,
                "receipt.inventory",
            )
            inventory = read_strict_object(inventory_path)
            artifact_bindings.extend(
                object_list(
                    inventory,
                    "unit_inventories",
                    "receipt.unit_inventories",
                )
            )
            artifact_bindings.extend(object_list(file_record, "png", "receipt.png"))
    return _validated_artifacts(artifact_bindings, evidence_root)


def _validated_artifacts(
    bindings: list[dict[str, JsonValue]],
    evidence_root: Path,
) -> list[dict[str, JsonValue]]:
    result: list[dict[str, JsonValue]] = []
    paths: set[str] = set()
    for binding in bindings:
        relative = string_value(binding, "path")
        if relative in paths:
            raise MetricError("artifact.path", relative)
        paths.add(relative)
        resolve_artifact_binding(binding, evidence_root, "receipt.artifact")
        result.append(
            {
                "path": relative,
                "sha256": string_value(binding, "sha256"),
            }
        )
    result.sort(key=lambda value: str(value["path"]))
    return result
