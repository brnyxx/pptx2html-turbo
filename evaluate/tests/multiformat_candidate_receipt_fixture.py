from __future__ import annotations

import hashlib
import json
from pathlib import Path

from evaluate.multiformat_candidate_attestation import canonical_payload
from evaluate.multiformat_capture_receipt import (
    receipt_artifacts_from_manifests,
)
from evaluate.multiformat_schema import (
    JsonValue,
    object_value,
    sha256_file,
    string_value,
)
from evaluate.multiformat_strict_json import read_strict_object
from evaluate.tests.multiformat_attestation_fixture import (
    create_test_verifier,
    write_signed_attestation,
)
from evaluate.tests.multiformat_metric_artifact_fixture import binding
from evaluate.tests.multiformat_metric_artifact_fixture import sha256


def write_candidate_receipt(
    root: Path,
    document_format: str,
    runtime_identity: Path,
    execution_log: Path,
    determinism_value: dict[str, JsonValue],
    runtime_artifacts: dict[str, JsonValue],
    *,
    run_nonce: str,
    project_revision: str,
    contract_hash: str,
    corpus_hash: str,
    evaluator_hash: str,
    oracle_hash: str,
) -> tuple[dict[str, JsonValue], dict[str, JsonValue]]:
    determinism = root / f"{document_format}-candidate-determinism.json"
    determinism.write_text(
        json.dumps(determinism_value, sort_keys=True),
        encoding="utf-8",
    )
    artifacts = receipt_artifacts_from_manifests(
        determinism,
        runtime_artifacts,
        root,
    )
    payload: dict[str, JsonValue] = {
        "schema_version": 1,
        "status": "PASS",
        "verifier_id": "test-verifier",
        "run_nonce": run_nonce,
        "project_revision": project_revision,
        "contract_sha256": contract_hash,
        "corpus_manifest_sha256": corpus_hash,
        "evaluator_manifest_sha256": evaluator_hash,
        "oracle_lock_sha256": oracle_hash,
        "runtime_identity_sha256": sha256_file(runtime_identity),
        "execution_log_sha256": sha256_file(execution_log),
        "determinism_sha256": sha256_file(determinism),
        "artifact_root_sha256": hashlib.sha256(
            canonical_payload({"artifacts": artifacts})
        ).hexdigest(),
        "artifacts": artifacts,
    }
    receipt = root / f"{document_format}-candidate-receipt.json"
    write_signed_attestation(
        receipt,
        create_test_verifier(root),
        payload,
    )
    return binding(root, determinism), binding(root, receipt)


def refresh_candidate_receipt(
    root: Path,
    metrics: dict[str, JsonValue],
) -> None:
    capture_binding = object_value(
        object_value(metrics, "bindings"),
        "candidate_capture",
    )
    capture_path = root / string_value(capture_binding, "path")
    capture = read_strict_object(capture_path)
    upstream_binding = object_value(capture, "upstream_manifest")
    upstream_path = root / string_value(upstream_binding, "path")
    upstream = read_strict_object(upstream_path)
    runtime_path = root / string_value(
        object_value(capture, "runtime_identity"),
        "path",
    )
    runtime = read_strict_object(runtime_path)
    execution_path = root / string_value(
        object_value(upstream, "execution_log"),
        "path",
    )
    determinism_binding, receipt_binding = write_candidate_receipt(
        root,
        string_value(capture, "format"),
        runtime_path,
        execution_path,
        object_value(metrics, "determinism"),
        object_value(runtime, "artifacts"),
        run_nonce=string_value(object_value(runtime, "tools"), "run_nonce"),
        project_revision=string_value(upstream, "project_revision"),
        contract_hash=string_value(upstream, "contract_sha256"),
        corpus_hash=string_value(upstream, "corpus_manifest_sha256"),
        evaluator_hash=string_value(upstream, "evaluator_manifest_sha256"),
        oracle_hash=string_value(upstream, "oracle_lock_sha256"),
    )
    for values in [upstream, capture]:
        values["determinism_manifest"] = determinism_binding
        values["execution_receipt"] = receipt_binding
    upstream_path.write_text(json.dumps(upstream, sort_keys=True), encoding="utf-8")
    upstream_binding["sha256"] = sha256(upstream_path)
    capture_path.write_text(json.dumps(capture, sort_keys=True), encoding="utf-8")
    capture_binding["sha256"] = sha256(capture_path)
