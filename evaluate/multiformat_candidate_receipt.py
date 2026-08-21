from __future__ import annotations

import hashlib
import subprocess
from pathlib import Path

from evaluate.multiformat_candidate_artifacts import (
    evidence_binding,
    write_canonical_json,
)
from evaluate.multiformat_candidate_attestation import (
    canonical_payload,
    verify_signed_payload,
)
from evaluate.multiformat_candidate_types import (
    CandidateCaptureError,
    CandidateRun,
)
from evaluate.multiformat_schema import (
    JsonValue,
    object_value,
    sha256_file,
    string_value,
)
from evaluate.multiformat_strict_json import read_strict_object
from evaluate.multiformat_subprocess import clean_subprocess_environment


class CandidateReceiptError(CandidateCaptureError):
    pass


def write_execution_receipt(
    evidence_root: Path,
    output_dir: Path,
    receipt_signer: Path,
    public_key: Path,
    openssl: Path,
    oracle_lock: Path,
    *,
    run_nonce: str,
    project_revision: str,
    contract_sha256: str,
    corpus_sha256: str,
    evaluator_sha256: str,
    oracle_lock_sha256: str,
    runtime_identity: Path,
    execution_log: Path,
    determinism: Path,
    runs: tuple[CandidateRun, CandidateRun],
    runtime_artifacts: dict[str, Path],
) -> Path:
    artifacts = _artifact_bindings(evidence_root, runs, runtime_artifacts)
    payload: dict[str, JsonValue] = {
        "schema_version": 1,
        "status": "PASS",
        "verifier_id": string_value(
            object_value(read_strict_object(oracle_lock), "sandbox_verifier"),
            "verifier_id",
        ),
        "run_nonce": run_nonce,
        "project_revision": project_revision,
        "contract_sha256": contract_sha256,
        "corpus_manifest_sha256": corpus_sha256,
        "evaluator_manifest_sha256": evaluator_sha256,
        "oracle_lock_sha256": oracle_lock_sha256,
        "runtime_identity_sha256": sha256_file(runtime_identity),
        "execution_log_sha256": sha256_file(execution_log),
        "determinism_sha256": sha256_file(determinism),
        "artifact_root_sha256": hashlib.sha256(
            canonical_payload({"artifacts": artifacts})
        ).hexdigest(),
        "artifacts": artifacts,
    }
    request = output_dir / "receipt-request.json"
    receipt = output_dir / "execution-receipt.json"
    write_canonical_json(request, payload)
    try:
        result = subprocess.run(
            [
                receipt_signer.as_posix(),
                "--request",
                request.as_posix(),
                "--output",
                receipt.as_posix(),
            ],
            check=False,
            capture_output=True,
            env=clean_subprocess_environment(),
            timeout=30,
        )
    except (OSError, subprocess.SubprocessError) as error:
        raise CandidateReceiptError("receipt signer failed") from error
    if (
        result.returncode != 0
        or len(result.stdout) > 1024 * 1024
        or len(result.stderr) > 1024 * 1024
    ):
        raise CandidateReceiptError("receipt signer rejected the capture")
    verify_signed_payload(
        receipt,
        public_key,
        openssl,
        oracle_lock,
        payload,
    )
    return receipt


def _artifact_bindings(
    evidence_root: Path,
    runs: tuple[CandidateRun, CandidateRun],
    runtime_artifacts: dict[str, Path],
) -> list[dict[str, JsonValue]]:
    paths = set(runtime_artifacts.values())
    for run in runs:
        for source in run.sources:
            paths.add(source.html)
            paths.add(source.inventory_manifest)
            for unit in source.units:
                paths.add(unit.png)
                paths.add(unit.inventory)
    bindings = [evidence_binding(evidence_root, path) for path in paths]
    bindings.sort(key=lambda value: str(value["path"]))
    if len({str(value["path"]) for value in bindings}) != len(bindings):
        raise CandidateReceiptError("receipt artifacts contain duplicate paths")
    return bindings
