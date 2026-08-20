from __future__ import annotations

import base64
import hashlib
import json
import os
import subprocess
import tempfile
from dataclasses import dataclass
from pathlib import Path

from evaluate.multiformat_candidate_types import CandidateCaptureError
from evaluate.multiformat_schema import (
    JsonValue,
    object_value,
    sha256_file,
    sha256_value,
    string_value,
)
from evaluate.multiformat_strict_json import read_strict_object


class CandidateAttestationError(CandidateCaptureError):
    pass


@dataclass(frozen=True, slots=True)
class VerifiedAttestation:
    verifier_id: str
    font_environment_sha256: str
    run_nonce: str


def verify_signed_payload(
    signed_path: Path,
    public_key_path: Path,
    openssl_path: Path,
    oracle_lock_path: Path,
    expected: dict[str, JsonValue],
) -> None:
    values = read_strict_object(signed_path.resolve(strict=True))
    signature_value = values.pop("signature", None)
    if not isinstance(signature_value, str) or not signature_value:
        raise CandidateAttestationError("signature is missing")
    if values != expected:
        raise CandidateAttestationError("signed payload mismatch")
    _verify_signature(
        values,
        signature_value,
        public_key_path,
        openssl_path,
        oracle_lock_path,
    )


def attestation_scope_sha256(
    contract_path: Path,
    corpus_path: Path,
    evaluator_path: Path,
    oracle_lock_path: Path,
) -> str:
    return attestation_scope_from_hashes(
        sha256_file(contract_path),
        sha256_file(corpus_path),
        sha256_file(evaluator_path),
        sha256_file(oracle_lock_path),
    )


def attestation_scope_from_hashes(
    contract_sha256: str,
    corpus_sha256: str,
    evaluator_sha256: str,
    oracle_lock_sha256: str,
) -> str:
    value = {
        "contract_sha256": contract_sha256,
        "corpus_sha256": corpus_sha256,
        "evaluator_sha256": evaluator_sha256,
        "oracle_lock_sha256": oracle_lock_sha256,
    }
    return hashlib.sha256(canonical_payload(value)).hexdigest()


def canonical_payload(value: dict[str, JsonValue]) -> bytes:
    return json.dumps(
        value,
        ensure_ascii=True,
        sort_keys=True,
        separators=(",", ":"),
    ).encode()


def verify_signed_attestation(
    attestation_path: Path,
    public_key_path: Path,
    openssl_path: Path,
    oracle_lock_path: Path,
    *,
    project_revision: str,
    scope_sha256: str,
) -> VerifiedAttestation:
    values = read_strict_object(attestation_path.resolve(strict=True))
    signature_value = values.pop("signature", None)
    if not isinstance(signature_value, str) or not signature_value:
        raise CandidateAttestationError("sandbox signature is missing")
    lock = read_strict_object(oracle_lock_path)
    verifier = object_value(lock, "sandbox_verifier")
    browser = object_value(lock, "browser")
    run_nonce = string_value(values, "run_nonce")
    if len(run_nonce) != 64 or any(
        character not in "0123456789abcdef" for character in run_nonce
    ):
        raise CandidateAttestationError("sandbox run nonce is invalid")
    expected = {
        "schema_version": 1,
        "status": "PASS",
        "network_isolation": "disabled",
        "golden_access": "denied",
        "project_revision": project_revision,
        "scope_sha256": scope_sha256,
        "font_environment_sha256": sha256_value(
            browser,
            "font_environment_sha256",
        ),
        "font_isolation": "locked-bundle-only",
        "run_nonce": run_nonce,
        "verifier_id": string_value(verifier, "verifier_id"),
    }
    if values != expected:
        raise CandidateAttestationError("sandbox attestation payload mismatch")
    _verify_signature(
        values,
        signature_value,
        public_key_path,
        openssl_path,
        oracle_lock_path,
    )
    return VerifiedAttestation(
        expected["verifier_id"],
        expected["font_environment_sha256"],
        run_nonce,
    )


def _verify_signature(
    values: dict[str, JsonValue],
    signature_value: str,
    public_key_path: Path,
    openssl_path: Path,
    oracle_lock_path: Path,
) -> None:
    lock = read_strict_object(oracle_lock_path)
    verifier = object_value(lock, "sandbox_verifier")
    public_key = public_key_path.resolve(strict=True)
    openssl = openssl_path.resolve(strict=True)
    if (
        string_value(verifier, "algorithm") != "ed25519"
        or sha256_file(public_key) != sha256_value(verifier, "public_key_sha256")
        or sha256_file(openssl) != sha256_value(verifier, "openssl_sha256")
    ):
        raise CandidateAttestationError("sandbox verifier lock mismatch")
    try:
        signature = base64.b64decode(signature_value, validate=True)
    except ValueError as error:
        raise CandidateAttestationError("sandbox signature is invalid") from error
    with tempfile.TemporaryDirectory(prefix="candidate-attestation-") as temp_dir:
        root = Path(temp_dir)
        payload_path = root / "payload.json"
        signature_path = root / "signature.bin"
        payload_path.write_bytes(canonical_payload(values))
        signature_path.write_bytes(signature)
        result = subprocess.run(
            [
                openssl.as_posix(),
                "pkeyutl",
                "-verify",
                "-pubin",
                "-inkey",
                public_key.as_posix(),
                "-rawin",
                "-in",
                payload_path.as_posix(),
                "-sigfile",
                signature_path.as_posix(),
            ],
            check=False,
            capture_output=True,
            env={"PATH": os.defpath},
            timeout=15,
        )
    if result.returncode != 0:
        raise CandidateAttestationError("sandbox signature verification failed")
