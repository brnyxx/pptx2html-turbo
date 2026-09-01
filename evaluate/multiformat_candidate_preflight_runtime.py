from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

from evaluate.multiformat_candidate_runtime_profile import (
    CandidateRuntimeProfile,
    CandidateRuntimeProfileError,
    require_profile_path,
)
from evaluate.multiformat_schema import sha256_file, sha256_value, string_value


@dataclass(frozen=True, slots=True)
class CandidateInputPaths:
    chromium: Path
    font_bundle: Path
    receipt_executor: Path
    sandbox_attestation: Path
    sandbox_public_key: Path
    openssl: Path


def resolve_candidate_input_paths(
    profile: CandidateRuntimeProfile,
    *,
    chromium: Path,
    font_bundle: Path,
    receipt_executor: Path,
    sandbox_attestation: Path,
    sandbox_public_key: Path,
    openssl: Path,
) -> CandidateInputPaths:
    key = sandbox_public_key.resolve(strict=True)
    openssl = openssl.resolve(strict=True)
    verifier = profile.sandbox_verifier
    if (
        string_value(verifier, "algorithm") != "ed25519"
        or not string_value(verifier, "verifier_id")
        or sha256_file(key) != sha256_value(verifier, "public_key_sha256")
        or sha256_file(openssl) != sha256_value(verifier, "openssl_sha256")
    ):
        raise CandidateRuntimeProfileError("candidate sandbox verifier lock mismatch")
    return CandidateInputPaths(
        require_profile_path(chromium, profile.chromium, "Chromium"),
        require_profile_path(font_bundle, profile.font_bundle, "font bundle"),
        require_profile_path(
            receipt_executor, profile.receipt_executor, "receipt executor"
        ),
        sandbox_attestation.resolve(strict=True),
        key,
        openssl,
    )


def require_candidate_evidence_root(
    evidence_root: Path, paths: tuple[Path, ...]
) -> None:
    root = evidence_root.resolve(strict=True)
    for path in paths:
        if not path.resolve(strict=True).is_relative_to(root):
            raise CandidateRuntimeProfileError(
                f"runtime evidence escapes evidence root: {path}"
            )


__all__ = [
    "CandidateInputPaths",
    "require_candidate_evidence_root",
    "resolve_candidate_input_paths",
]
