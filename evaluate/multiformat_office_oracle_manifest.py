from __future__ import annotations

from pathlib import Path

from evaluate.multiformat_candidate_artifacts import (
    evidence_binding,
    write_canonical_json,
)
from evaluate.multiformat_schema import JsonValue, sha256_file


def write_office_oracle_manifests(
    output_dir: Path,
    *,
    document_format: str,
    producer: str,
    project_revision: str,
    contract_sha256: str,
    corpus_sha256: str,
    evaluator_sha256: str,
    oracle_lock_sha256: str,
    runtime_identity: Path,
    execution_log: Path,
    batch_manifest: Path,
    receipt: Path,
    units: list[dict[str, JsonValue]],
) -> Path:
    root = output_dir.resolve(strict=True)
    runtime_binding = evidence_binding(root, runtime_identity)
    batch_binding = evidence_binding(root, batch_manifest)
    receipt_binding = evidence_binding(root, receipt)
    runtime_hash = sha256_file(runtime_identity)
    shared: dict[str, JsonValue] = {
        "schema_version": 1,
        "status": "READY",
        "role": "oracle",
        "format": document_format,
        "producer": producer,
        "runtime_sha256": runtime_hash,
        "runtime_identity": runtime_binding,
        "contract_sha256": contract_sha256,
        "corpus_manifest_sha256": corpus_sha256,
        "evaluator_manifest_sha256": evaluator_sha256,
        "oracle_lock_sha256": oracle_lock_sha256,
        "units": units,
        "files": [],
        "office_batch_manifest": batch_binding,
        "execution_receipt": receipt_binding,
    }
    upstream = root / "upstream.json"
    write_canonical_json(
        upstream,
        {
            **shared,
            "project_revision": project_revision,
            "execution_log": evidence_binding(root, execution_log),
        },
    )
    capture = root / "capture.json"
    write_canonical_json(
        capture,
        {
            **shared,
            "network_isolation": "disabled",
            "rendering": _rendering(document_format),
            "upstream_manifest": evidence_binding(root, upstream),
        },
    )
    return capture


def _rendering(document_format: str) -> dict[str, JsonValue]:
    if document_format in {"ppt", "pptx"}:
        return {"dpi": None, "width": 960, "height": 540}
    return {"dpi": 144, "width": None, "height": None}
