from __future__ import annotations

import hashlib
import subprocess
from pathlib import Path

from evaluate.multiformat_candidate_artifacts import (
    evidence_binding,
    write_canonical_json,
)
from evaluate.multiformat_candidate_attestation import (
    CandidateAttestationError,
    canonical_payload,
    verify_signed_payload,
)
from evaluate.multiformat_schema import (
    JsonValue,
    object_value,
    sha256_file,
    string_value,
)
from evaluate.multiformat_strict_json import read_strict_object
from evaluate.multiformat_subprocess import clean_subprocess_environment


class OfficeOracleReceiptError(Exception):
    pass


def write_office_oracle_receipt(
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
    batch_manifest: Path,
    runtime_identity: Path,
    execution_log: Path,
    artifacts: list[Path],
) -> Path:
    payload = _receipt_payload(
        evidence_root,
        oracle_lock,
        run_nonce=run_nonce,
        project_revision=project_revision,
        contract_sha256=contract_sha256,
        corpus_sha256=corpus_sha256,
        evaluator_sha256=evaluator_sha256,
        oracle_lock_sha256=oracle_lock_sha256,
        batch_manifest=batch_manifest,
        runtime_identity=runtime_identity,
        execution_log=execution_log,
        artifacts=artifacts,
    )
    request = output_dir / "office-oracle-receipt-request.json"
    receipt = output_dir / "office-oracle-execution-receipt.json"
    write_canonical_json(request, payload)
    try:
        result = subprocess.run(
            [
                receipt_signer.resolve(strict=True).as_posix(),
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
        raise OfficeOracleReceiptError("office receipt signer failed") from error
    if (
        result.returncode != 0
        or len(result.stdout) > 1024 * 1024
        or len(result.stderr) > 1024 * 1024
    ):
        raise OfficeOracleReceiptError("office receipt signer rejected capture")
    _verify_receipt(
        receipt,
        public_key,
        openssl,
        oracle_lock,
        payload,
    )
    return receipt


def validate_office_oracle_receipt(
    *,
    receipt: Path,
    public_key: Path,
    openssl: Path,
    oracle_lock: Path,
    run_nonce: str,
    project_revision: str,
    contract_sha256: str,
    corpus_sha256: str,
    evaluator_sha256: str,
    oracle_lock_sha256: str,
    batch_manifest: Path,
    runtime_identity: Path,
    execution_log: Path,
    artifacts: list[Path],
    evidence_root: Path,
) -> None:
    payload = _receipt_payload(
        evidence_root,
        oracle_lock,
        run_nonce=run_nonce,
        project_revision=project_revision,
        contract_sha256=contract_sha256,
        corpus_sha256=corpus_sha256,
        evaluator_sha256=evaluator_sha256,
        oracle_lock_sha256=oracle_lock_sha256,
        batch_manifest=batch_manifest,
        runtime_identity=runtime_identity,
        execution_log=execution_log,
        artifacts=artifacts,
    )
    _verify_receipt(
        receipt,
        public_key,
        openssl,
        oracle_lock,
        payload,
    )


def _receipt_payload(
    evidence_root: Path,
    oracle_lock: Path,
    *,
    run_nonce: str,
    project_revision: str,
    contract_sha256: str,
    corpus_sha256: str,
    evaluator_sha256: str,
    oracle_lock_sha256: str,
    batch_manifest: Path,
    runtime_identity: Path,
    execution_log: Path,
    artifacts: list[Path],
) -> dict[str, JsonValue]:
    if len(run_nonce) != 64 or any(
        character not in "0123456789abcdef" for character in run_nonce
    ):
        raise OfficeOracleReceiptError("office run nonce is invalid")
    bindings = _artifact_bindings(evidence_root, artifacts)
    verifier = object_value(
        read_strict_object(oracle_lock),
        "office_oracle_verifier",
    )
    return {
        "schema_version": 1,
        "status": "PASS",
        "verifier_id": string_value(verifier, "verifier_id"),
        "run_nonce": run_nonce,
        "project_revision": project_revision,
        "contract_sha256": contract_sha256,
        "corpus_manifest_sha256": corpus_sha256,
        "evaluator_manifest_sha256": evaluator_sha256,
        "oracle_lock_sha256": oracle_lock_sha256,
        "batch_manifest_sha256": sha256_file(batch_manifest),
        "runtime_identity_sha256": sha256_file(runtime_identity),
        "execution_log_sha256": sha256_file(execution_log),
        "artifact_root_sha256": hashlib.sha256(
            canonical_payload({"artifacts": bindings})
        ).hexdigest(),
        "artifacts": bindings,
    }


def _artifact_bindings(
    evidence_root: Path,
    artifacts: list[Path],
) -> list[dict[str, JsonValue]]:
    root = evidence_root.resolve(strict=True)
    resolved = {path.resolve(strict=True) for path in artifacts}
    if len(resolved) != len(artifacts):
        raise OfficeOracleReceiptError("office artifacts contain duplicates")
    try:
        bindings = [
            evidence_binding(root, path)
            for path in sorted(resolved, key=lambda item: item.as_posix())
        ]
    except (OSError, ValueError) as error:
        raise OfficeOracleReceiptError("office artifact is invalid") from error
    return bindings


def _verify_receipt(
    receipt: Path,
    public_key: Path,
    openssl: Path,
    oracle_lock: Path,
    payload: dict[str, JsonValue],
) -> None:
    try:
        verify_signed_payload(
            receipt,
            public_key,
            openssl,
            oracle_lock,
            payload,
            verifier_field="office_oracle_verifier",
        )
    except (CandidateAttestationError, OSError, ValueError) as error:
        raise OfficeOracleReceiptError(
            "office artifact receipt verification failed"
        ) from error
