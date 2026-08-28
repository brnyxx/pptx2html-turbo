"""Sign one post-lock candidate sandbox attestation."""

from __future__ import annotations

import argparse
import base64
import hashlib
import sys
from collections.abc import Sequence
from pathlib import Path
from typing import Final

from cryptography.hazmat.primitives import serialization

from evaluate.multiformat_candidate_attestation import (
    attestation_scope_sha256,
    canonical_payload,
)
from evaluate.multiformat_candidate_attestation_signing_io import (
    CandidateAttestationSignError,
    claim_nonce,
    exclusive_write,
    load_private_key,
    new_output,
)
from evaluate.multiformat_candidate_sandbox import (
    CandidateSandbox,
    CandidateSandboxError,
    network_probe_value,
    observe_network_control,
    observe_sandbox,
    oracle_probe_value,
    resolve_locked_sandbox,
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

VERSION: Final = "sign-multiformat-candidate-attestation 3"


def sign_candidate_attestation(
    evidence_root: Path,
    output: Path,
    private_key: Path,
    outer_lock: Path,
    contract: Path,
    corpus: Path,
    evaluator: Path,
    *,
    oracle_root: Path,
    oracle_sentinel: Path,
    run_nonce: str,
) -> Path:
    """Derive the final lock scope and exclusively publish a signed payload."""
    try:
        root = evidence_root.resolve(strict=True)
        destination = new_output(root, output)
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
        sandbox_executable, sandbox_profile, libreoffice, chromium = (
            resolve_locked_sandbox(lock, root)
        )
        resolved_oracle_root = oracle_root.resolve(strict=True)
        sentinel = oracle_sentinel.resolve(strict=True)
        sandbox = CandidateSandbox(
            sandbox_executable,
            sandbox_profile,
            libreoffice,
            chromium,
            resolved_oracle_root,
            sentinel,
        )
        observe_network_control()
        observe_sandbox(sandbox)
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
        key = load_private_key(private_key)
        public_pem = key.public_key().public_bytes(
            serialization.Encoding.PEM,
            serialization.PublicFormat.SubjectPublicKeyInfo,
        )
        if hashlib.sha256(public_pem).hexdigest() != sha256_value(
            verifier, "public_key_sha256"
        ):
            raise CandidateAttestationSignError("candidate signing key is not locked")
        payload: dict[str, JsonValue] = {
            "schema_version": 3,
            "status": "PASS",
            "network_isolation": True,
            "golden_access": "denied",
            "sandbox_executable": sandbox.executable_binding(root),
            "sandbox_profile": sandbox.profile_binding(root),
            "network_probe": network_probe_value(),
            "oracle_probe": oracle_probe_value(root, sandbox),
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
        claim_nonce(root, payload)
        value: dict[str, JsonValue] = {
            **payload,
            "signature": base64.b64encode(signature).decode("ascii"),
        }
        exclusive_write(destination, canonical_payload(value) + b"\n", 0o644)
        return destination
    except CandidateAttestationSignError:
        raise
    except CandidateSandboxError as error:
        raise CandidateAttestationSignError(str(error)) from error
    except (OSError, TypeError, UnicodeError, ValueError) as error:
        raise CandidateAttestationSignError(
            "candidate attestation signing failed"
        ) from error


def _bound_path(root: Path, binding: dict[str, JsonValue]) -> Path:
    path = resolve_evidence_path(root, string_value(binding, "path"))
    if sha256_file(path) != sha256_value(binding, "sha256"):
        raise CandidateAttestationSignError("candidate bound artifact differs")
    return path


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
    parser.add_argument("--oracle-root", type=Path, required=True)
    parser.add_argument("--oracle-sentinel", type=Path, required=True)
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
            oracle_root=args.oracle_root,
            oracle_sentinel=args.oracle_sentinel,
            run_nonce=args.run_nonce,
        )
    except (OSError, TypeError, ValueError):
        sys.stderr.write("candidate attestation signing failed\n")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
