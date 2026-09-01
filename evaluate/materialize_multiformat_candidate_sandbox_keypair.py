"""Materialize a candidate-only Ed25519 PEM keypair."""

from __future__ import annotations

import argparse
import os
import sys
from collections.abc import Sequence
from pathlib import Path
from typing import Final

from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric.ed25519 import (
    Ed25519PrivateKey,
    Ed25519PublicKey,
)

VERSION: Final = "materialize-multiformat-candidate-sandbox-keypair 1"


class CandidateKeypairError(ValueError):
    """Candidate sandbox key material cannot be published safely."""


def materialize_candidate_sandbox_keypair(
    project_root: Path,
    evidence_root: Path,
    private_key: Path,
    public_key: Path,
    outer_public_key: Path,
) -> tuple[Path, Path]:
    """Create a distinct key, keeping private bytes outside project evidence."""
    private_created = False
    try:
        project = project_root.resolve(strict=True)
        root = evidence_root.resolve(strict=True)
        private_parent = private_key.parent.resolve(strict=True)
        private = private_parent / private_key.name
        public_parent = public_key.parent.resolve(strict=True)
        public = public_parent / public_key.name
        if private.is_relative_to(project) or private.is_relative_to(root):
            raise CandidateKeypairError("candidate private key must remain external")
        if not public.is_relative_to(root):
            raise CandidateKeypairError("candidate public key escapes evidence root")
        if (
            private.exists()
            or private.is_symlink()
            or public.exists()
            or public.is_symlink()
        ):
            raise CandidateKeypairError("candidate key destination already exists")
        outer = _load_public_key(outer_public_key)
        candidate = Ed25519PrivateKey.generate()
        if candidate.public_key().public_bytes_raw() == outer.public_bytes_raw():
            raise CandidateKeypairError("candidate key must differ from outer signer")
        _exclusive_write(private, _private_pem(candidate), 0o600)
        private_created = True
        _exclusive_write(public, _public_pem(candidate), 0o644)
        return private, public
    except CandidateKeypairError:
        raise
    except (OSError, TypeError, ValueError) as error:
        raise CandidateKeypairError(
            "candidate keypair materialization failed"
        ) from error
    finally:
        if private_created and not public_key.exists():
            private_key.unlink(missing_ok=True)


def _load_public_key(path: Path) -> Ed25519PublicKey:
    value = path.resolve(strict=True).read_bytes()
    if len(value) == 32:
        return Ed25519PublicKey.from_public_bytes(value)
    loaded = serialization.load_pem_public_key(value)
    if not isinstance(loaded, Ed25519PublicKey):
        raise CandidateKeypairError("outer public key is not Ed25519")
    return loaded


def _private_pem(key: Ed25519PrivateKey) -> bytes:
    return key.private_bytes(
        serialization.Encoding.PEM,
        serialization.PrivateFormat.PKCS8,
        serialization.NoEncryption(),
    )


def _public_pem(key: Ed25519PrivateKey) -> bytes:
    return key.public_key().public_bytes(
        serialization.Encoding.PEM,
        serialization.PublicFormat.SubjectPublicKeyInfo,
    )


def _exclusive_write(path: Path, value: bytes, mode: int) -> None:
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, mode)
    try:
        os.write(descriptor, value)
        os.fsync(descriptor)
    finally:
        os.close(descriptor)


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Create a distinct candidate sandbox Ed25519 keypair."
    )
    parser.add_argument("--version", action="version", version=VERSION)
    parser.add_argument("--project-root", type=Path, required=True)
    parser.add_argument("--evidence-root", type=Path, required=True)
    parser.add_argument("--private-key", type=Path, required=True)
    parser.add_argument("--public-key", type=Path, required=True)
    parser.add_argument("--outer-public-key", type=Path, required=True)
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    args = _parser().parse_args(argv)
    try:
        materialize_candidate_sandbox_keypair(
            args.project_root,
            args.evidence_root,
            args.private_key,
            args.public_key,
            args.outer_public_key,
        )
    except (OSError, TypeError, ValueError):
        sys.stderr.write("candidate sandbox keypair materialization failed\n")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
