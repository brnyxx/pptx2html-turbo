"""Create the raw Ed25519 keypair used by portable reference receipts."""

from __future__ import annotations

import argparse
import os
import stat
import sys
from collections.abc import Sequence
from pathlib import Path
from typing import Final

from cryptography.exceptions import InvalidSignature
from cryptography.hazmat.primitives.asymmetric.ed25519 import (
    Ed25519PrivateKey,
    Ed25519PublicKey,
)

VERSION: Final = "materialize-multiformat-portable-reference-keypair 1"
_PROBE: Final = b"multiformat-portable-reference-keypair-v1"
_FileIdentity = tuple[int, int]


class PortableReferenceKeypairError(ValueError):
    """Raw portable reference key material cannot be created safely."""


def load_raw_reference_private_key(path: Path) -> Ed25519PrivateKey:
    """Load one no-follow, exact-mode, raw 32-byte Ed25519 private key."""
    value = _read_raw_key(path, 0o600, "private")
    return Ed25519PrivateKey.from_private_bytes(value)


def materialize_portable_reference_keypair(
    project_root: Path,
    evidence_root: Path,
    private_key: Path,
    public_key: Path,
) -> tuple[Path, Path]:
    """Create and immediately prove one raw reference receipt keypair."""
    private_identity: _FileIdentity | None = None
    public_identity: _FileIdentity | None = None
    private: Path | None = None
    public: Path | None = None
    complete = False
    try:
        if project_root.is_symlink() or evidence_root.is_symlink():
            raise PortableReferenceKeypairError("keypair root must not be a symlink")
        project = project_root.resolve(strict=True)
        root = evidence_root.resolve(strict=True)
        if not project.is_dir() or not root.is_dir():
            raise PortableReferenceKeypairError("keypair root is not a directory")
        private = _new_private_path(private_key, project, root)
        public = _new_public_path(public_key, root)
        key = Ed25519PrivateKey.generate()
        private_identity = _exclusive_write(private, key.private_bytes_raw(), 0o600)
        public_identity = _exclusive_write(
            public, key.public_key().public_bytes_raw(), 0o644
        )
        loaded_private = load_raw_reference_private_key(private)
        public_bytes = _read_raw_key(public, 0o644, "public")
        if loaded_private.public_key().public_bytes_raw() != public_bytes:
            raise PortableReferenceKeypairError("raw reference keypair differs")
        signature = loaded_private.sign(_PROBE)
        Ed25519PublicKey.from_public_bytes(public_bytes).verify(signature, _PROBE)
        complete = True
        return private, public
    except PortableReferenceKeypairError:
        raise
    except (InvalidSignature, OSError, TypeError, ValueError) as error:
        raise PortableReferenceKeypairError(
            "portable reference keypair materialization failed"
        ) from error
    finally:
        if not complete:
            if public is not None and public_identity is not None:
                _unlink_owned(public, public_identity)
            if private is not None and private_identity is not None:
                _unlink_owned(private, private_identity)


def _new_private_path(path: Path, project: Path, root: Path) -> Path:
    if path.exists() or path.is_symlink():
        raise PortableReferenceKeypairError("reference key destination exists")
    parent = path.parent.resolve(strict=True)
    destination = parent / path.name
    if destination.is_relative_to(project) or destination.is_relative_to(root):
        raise PortableReferenceKeypairError("reference private key is not external")
    return destination


def _new_public_path(path: Path, root: Path) -> Path:
    if path.exists() or path.is_symlink():
        raise PortableReferenceKeypairError("reference key destination exists")
    unresolved = path.resolve(strict=False)
    if not unresolved.is_relative_to(root) or unresolved == root:
        raise PortableReferenceKeypairError(
            "reference public key escapes evidence root"
        )
    unresolved.parent.mkdir(parents=True, exist_ok=True)
    parent = unresolved.parent.resolve(strict=True)
    destination = parent / unresolved.name
    if not destination.is_relative_to(root):
        raise PortableReferenceKeypairError(
            "reference public key escapes evidence root"
        )
    return destination


def _exclusive_write(path: Path, value: bytes, mode: int) -> _FileIdentity:
    flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, "O_NOFOLLOW", 0)
    descriptor = os.open(path, flags, mode)
    try:
        os.fchmod(descriptor, mode)
        view = memoryview(value)
        while view:
            written = os.write(descriptor, view)
            if written <= 0:
                raise OSError("raw key write made no progress")
            view = view[written:]
        os.fsync(descriptor)
        info = os.fstat(descriptor)
        return info.st_dev, info.st_ino
    finally:
        os.close(descriptor)


def _read_raw_key(path: Path, mode: int, label: str) -> bytes:
    if path.is_symlink():
        raise PortableReferenceKeypairError(f"reference {label} key is a symlink")
    flags = os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0)
    descriptor = os.open(path, flags)
    try:
        info = os.fstat(descriptor)
        if (
            not stat.S_ISREG(info.st_mode)
            or stat.S_IMODE(info.st_mode) != mode
            or info.st_size != 32
        ):
            raise PortableReferenceKeypairError(
                f"reference {label} key format or permissions differ"
            )
        value = os.read(descriptor, 33)
        if len(value) != 32:
            raise PortableReferenceKeypairError(
                f"reference {label} key must be exactly 32 bytes"
            )
        return value
    finally:
        os.close(descriptor)


def _unlink_owned(path: Path, identity: _FileIdentity) -> None:
    try:
        info = path.lstat()
    except FileNotFoundError:
        return
    if not path.is_symlink() and (info.st_dev, info.st_ino) == identity:
        path.unlink()


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Create a raw portable reference receipt Ed25519 keypair."
    )
    parser.add_argument("--version", action="version", version=VERSION)
    parser.add_argument("--project-root", type=Path, required=True)
    parser.add_argument("--evidence-root", type=Path, required=True)
    parser.add_argument("--private-key", type=Path, required=True)
    parser.add_argument("--public-key", type=Path, required=True)
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    args = _parser().parse_args(argv)
    try:
        materialize_portable_reference_keypair(
            args.project_root,
            args.evidence_root,
            args.private_key,
            args.public_key,
        )
    except (OSError, TypeError, ValueError):
        sys.stderr.write("portable reference keypair materialization failed\n")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
