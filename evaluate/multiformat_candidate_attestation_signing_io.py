from __future__ import annotations

import hashlib
import os
import stat
from pathlib import Path

from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

from evaluate.multiformat_candidate_attestation import canonical_payload
from evaluate.multiformat_schema import JsonValue


class CandidateAttestationSignError(ValueError):
    """A candidate attestation cannot be derived or signed safely."""


def load_private_key(path: Path) -> Ed25519PrivateKey:
    if path.is_symlink():
        raise CandidateAttestationSignError(
            "candidate private key must not be a symlink"
        )
    resolved = path.resolve(strict=True)
    info = resolved.stat()
    if (
        not stat.S_ISREG(info.st_mode)
        or info.st_size > 4096
        or stat.S_IMODE(info.st_mode) & 0o077
    ):
        raise CandidateAttestationSignError("candidate private key permissions differ")
    loaded = serialization.load_pem_private_key(resolved.read_bytes(), password=None)
    if not isinstance(loaded, Ed25519PrivateKey):
        raise CandidateAttestationSignError("candidate private key is not Ed25519")
    return loaded


def new_output(root: Path, supplied: Path) -> Path:
    if supplied.exists() or supplied.is_symlink():
        raise CandidateAttestationSignError("candidate attestation already exists")
    unresolved = supplied.resolve(strict=False)
    if not unresolved.is_relative_to(root) or unresolved == root:
        raise CandidateAttestationSignError(
            "candidate attestation escapes evidence root"
        )
    unresolved.parent.mkdir(parents=True, exist_ok=True)
    destination = unresolved.parent.resolve(strict=True) / unresolved.name
    if not destination.is_relative_to(root):
        raise CandidateAttestationSignError(
            "candidate attestation escapes evidence root"
        )
    return destination


def claim_nonce(root: Path, payload: dict[str, JsonValue]) -> None:
    identity = {
        field: payload[field] for field in ("verifier_id", "scope_sha256", "run_nonce")
    }
    digest = hashlib.sha256(canonical_payload(identity)).hexdigest()
    claims = root / ".candidate-attestation-nonces"
    if claims.is_symlink():
        raise CandidateAttestationSignError("candidate nonce ledger is unsafe")
    claims.mkdir(exist_ok=True)
    resolved = claims.resolve(strict=True)
    if not resolved.is_dir() or not resolved.is_relative_to(root):
        raise CandidateAttestationSignError("candidate nonce ledger is unsafe")
    exclusive_write(resolved / digest, b"CLAIMED\n", 0o600)


def exclusive_write(path: Path, value: bytes, mode: int) -> None:
    try:
        descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, mode)
    except FileExistsError as error:
        raise CandidateAttestationSignError(
            "candidate nonce or output was reused"
        ) from error
    try:
        os.write(descriptor, value)
        os.fsync(descriptor)
    finally:
        os.close(descriptor)
