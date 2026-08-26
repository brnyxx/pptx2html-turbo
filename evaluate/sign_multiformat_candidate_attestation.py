"""Sign one post-lock candidate sandbox attestation."""

from __future__ import annotations

import argparse
import base64
import hashlib
import os
import stat
import sys
from collections.abc import Sequence
from pathlib import Path
from typing import Final

from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

from evaluate.multiformat_candidate_attestation import (
    attestation_scope_sha256,
    canonical_payload,
)
from evaluate.multiformat_evidence import resolve_evidence_path
from evaluate.multiformat_portable_lock import validate_reference_lock
from evaluate.multiformat_schema import (
    JsonValue,
    object_value,
    sha256_file,
    sha256_value,
    string_value,
)
from evaluate.multiformat_strict_json import read_strict_object

VERSION: Final = "sign-multiformat-candidate-attestation 1"


class CandidateAttestationSignError(ValueError):
    """A candidate attestation cannot be derived or signed safely."""


def sign_candidate_attestation(
    evidence_root: Path,
    output: Path,
    private_key: Path,
    outer_lock: Path,
    contract: Path,
    corpus: Path,
    evaluator: Path,
    *,
    run_nonce: str,
) -> Path:
    """Derive the final lock scope and exclusively publish a signed payload."""
    try:
        root = evidence_root.resolve(strict=True)
        destination = _new_output(root, output)
        lock_path = outer_lock.resolve(strict=True)
        if not lock_path.is_relative_to(root):
            raise CandidateAttestationSignError("candidate lock escapes evidence root")
        validate_reference_lock(lock_path, root)
        lock = read_strict_object(lock_path)
        scope = object_value(lock, "scope")
        bound = {
            "contract": _bound_path(root, object_value(scope, "contract")),
            "corpus": _bound_path(root, object_value(scope, "corpus")),
            "evaluator": _bound_path(root, object_value(scope, "evaluator")),
        }
        supplied = {
            "contract": contract.resolve(strict=True),
            "corpus": corpus.resolve(strict=True),
            "evaluator": evaluator.resolve(strict=True),
        }
        if supplied != bound:
            raise CandidateAttestationSignError("candidate attestation scope differs")
        if len(run_nonce) != 64 or any(
            character not in "0123456789abcdef" for character in run_nonce
        ):
            raise CandidateAttestationSignError("candidate run nonce is malformed")
        runtime_lock = read_strict_object(
            _bound_path(root, object_value(lock, "candidate_runtime_lock"))
        )
        browser = object_value(lock, "browser")
        browser_lock = read_strict_object(
            _bound_path(root, object_value(browser, "lock"))
        )
        verifier = object_value(runtime_lock, "sandbox_verifier")
        if string_value(verifier, "algorithm") != "ed25519":
            raise CandidateAttestationSignError("candidate verifier algorithm differs")
        key = _load_private_key(private_key)
        public_pem = key.public_key().public_bytes(
            serialization.Encoding.PEM,
            serialization.PublicFormat.SubjectPublicKeyInfo,
        )
        if hashlib.sha256(public_pem).hexdigest() != sha256_value(
            verifier, "public_key_sha256"
        ):
            raise CandidateAttestationSignError("candidate signing key is not locked")
        payload: dict[str, JsonValue] = {
            "schema_version": 1,
            "status": "PASS",
            "network_isolation": True,
            "golden_access": "denied",
            "project_revision": string_value(scope, "project_revision"),
            "font_environment_sha256": sha256_value(
                browser_lock, "font_environment_sha256"
            ),
            "font_isolation": "locked-bundle-only",
            "run_nonce": run_nonce,
            "verifier_id": string_value(verifier, "verifier_id"),
            "scope_sha256": attestation_scope_sha256(
                bound["contract"], bound["corpus"], bound["evaluator"], lock_path
            ),
        }
        payload_bytes = canonical_payload(payload)
        signature = key.sign(payload_bytes)
        key.public_key().verify(signature, payload_bytes)
        _claim_nonce(root, payload)
        value: dict[str, JsonValue] = {
            **payload,
            "signature": base64.b64encode(signature).decode("ascii"),
        }
        _exclusive_write(destination, canonical_payload(value) + b"\n", 0o644)
        return destination
    except CandidateAttestationSignError:
        raise
    except (OSError, TypeError, UnicodeError, ValueError) as error:
        raise CandidateAttestationSignError(
            "candidate attestation signing failed"
        ) from error


def _bound_path(root: Path, binding: dict[str, JsonValue]) -> Path:
    path = resolve_evidence_path(root, string_value(binding, "path"))
    if sha256_file(path) != sha256_value(binding, "sha256"):
        raise CandidateAttestationSignError("candidate bound artifact differs")
    return path


def _load_private_key(path: Path) -> Ed25519PrivateKey:
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


def _new_output(root: Path, supplied: Path) -> Path:
    if supplied.exists() or supplied.is_symlink():
        raise CandidateAttestationSignError("candidate attestation already exists")
    unresolved = supplied.resolve(strict=False)
    if not unresolved.is_relative_to(root) or unresolved == root:
        raise CandidateAttestationSignError(
            "candidate attestation escapes evidence root"
        )
    unresolved.parent.mkdir(parents=True, exist_ok=True)
    parent = unresolved.parent.resolve(strict=True)
    destination = parent / unresolved.name
    if not destination.is_relative_to(root):
        raise CandidateAttestationSignError(
            "candidate attestation escapes evidence root"
        )
    return destination


def _claim_nonce(root: Path, payload: dict[str, JsonValue]) -> None:
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
    _exclusive_write(resolved / digest, b"CLAIMED\n", 0o600)


def _exclusive_write(path: Path, value: bytes, mode: int) -> None:
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


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Sign a final-lock-scoped candidate sandbox attestation."
    )
    parser.add_argument("--version", action="version", version=VERSION)
    parser.add_argument("--evidence-root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--private-key", type=Path, required=True)
    parser.add_argument("--outer-lock", type=Path, required=True)
    parser.add_argument("--contract", type=Path, required=True)
    parser.add_argument("--corpus", type=Path, required=True)
    parser.add_argument("--evaluator", type=Path, required=True)
    parser.add_argument("--run-nonce", required=True)
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    args = _parser().parse_args(argv)
    try:
        sign_candidate_attestation(
            args.evidence_root,
            args.output,
            args.private_key,
            args.outer_lock,
            args.contract,
            args.corpus,
            args.evaluator,
            run_nonce=args.run_nonce,
        )
    except (OSError, TypeError, ValueError):
        sys.stderr.write("candidate attestation signing failed\n")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
