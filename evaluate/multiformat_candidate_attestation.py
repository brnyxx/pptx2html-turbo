from __future__ import annotations

import hashlib
import json
from dataclasses import dataclass
from pathlib import Path

from evaluate.multiformat_candidate_runtime_profile import CandidateRuntimeProfile
from evaluate.multiformat_candidate_sandbox import (
    CandidateSandbox,
    network_probe_value,
    oracle_probe_value,
    resolve_attested_sandbox,
)
from evaluate.multiformat_candidate_signature import verify_ed25519_json
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
    sandbox: CandidateSandbox | None = None


def verify_signed_payload(
    signed_path: Path,
    public_key_path: Path,
    openssl_path: Path,
    oracle_lock_path: Path,
    expected: dict[str, JsonValue],
    *,
    verifier_field: str = "sandbox_verifier",
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
        verifier_field,
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
    value: dict[str, JsonValue] = {
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


def verify_candidate_attestation(
    profile: CandidateRuntimeProfile,
    attestation_path: Path,
    public_key_path: Path,
    openssl_path: Path,
    oracle_lock_path: Path,
    *,
    project_revision: str,
    scope_sha256: str,
) -> VerifiedAttestation:
    if not profile.portable:
        return verify_signed_attestation(
            attestation_path,
            public_key_path,
            openssl_path,
            oracle_lock_path,
            project_revision=project_revision,
            scope_sha256=scope_sha256,
        )
    values = read_strict_object(attestation_path.resolve(strict=True))
    signature = values.pop("signature", None)
    if not isinstance(signature, str) or not signature:
        raise CandidateAttestationError("portable sandbox signature is missing")
    nonce = sha256_value(values, "run_nonce")
    font_environment = sha256_value(values, "font_environment_sha256")
    verifier_id = string_value(profile.sandbox_verifier, "verifier_id")
    if (
        profile.evidence_root is None
        or profile.sandbox_executable is None
        or profile.sandbox_profile is None
        or profile.libreoffice is None
    ):
        raise CandidateAttestationError("portable sandbox lock binding is missing")
    sandbox = resolve_attested_sandbox(
        values,
        profile.evidence_root,
        (
            profile.sandbox_executable,
            profile.sandbox_profile,
            profile.libreoffice,
        ),
    )
    expected: dict[str, JsonValue] = {
        "schema_version": 3,
        "status": "PASS",
        "network_isolation": True,
        "golden_access": "denied",
        "sandbox_executable": sandbox.executable_binding(profile.evidence_root),
        "sandbox_profile": sandbox.profile_binding(profile.evidence_root),
        "network_probe": network_probe_value(),
        "oracle_probe": oracle_probe_value(profile.evidence_root, sandbox),
        "project_revision": project_revision,
        "font_environment_sha256": font_environment,
        "font_isolation": "locked-bundle-only",
        "run_nonce": nonce,
        "verifier_id": verifier_id,
        "scope_sha256": scope_sha256,
    }
    if values != expected:
        raise CandidateAttestationError("portable sandbox attestation mismatch")
    if font_environment != sha256_value(
        profile.browser_lock, "font_environment_sha256"
    ):
        raise CandidateAttestationError("portable sandbox font environment differs")
    verify_ed25519_json(
        canonical_payload(values),
        signature,
        public_key_path,
        openssl_path,
        profile.sandbox_verifier,
        "candidate sandbox verifier",
    )
    return VerifiedAttestation(verifier_id, font_environment, nonce, sandbox)


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
    expected: dict[str, JsonValue] = {
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
        string_value(expected, "verifier_id"),
        sha256_value(expected, "font_environment_sha256"),
        run_nonce,
    )


def _verify_signature(
    values: dict[str, JsonValue],
    signature_value: str,
    public_key_path: Path,
    openssl_path: Path,
    oracle_lock_path: Path,
    verifier_field: str = "sandbox_verifier",
) -> None:
    lock = read_strict_object(oracle_lock_path)
    verifier = object_value(lock, verifier_field)
    verify_ed25519_json(
        canonical_payload(values),
        signature_value,
        public_key_path,
        openssl_path,
        verifier,
        verifier_field,
    )
