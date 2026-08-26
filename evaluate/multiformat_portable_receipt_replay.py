"""Persistent create-only replay claims for portable receipts."""

from __future__ import annotations

import hashlib
import os
import stat
from pathlib import Path
from typing import Final, cast

from evaluate.jcs import canonicalize
from evaluate.multiformat_schema import JsonValue
from evaluate.multiformat_strict_json import parse_strict_object_bytes

_CLAIM_DIRECTORY: Final = ".portable-receipt-claims"


class PortableReceiptReplayError(ValueError):
    """A replay claim is unsafe, corrupt, or bound to another receipt path."""


def artifact_root_sha256(artifacts: list[dict[str, JsonValue]]) -> str:
    """Return the stable root used to distinguish signed artifact sets."""
    value = cast(JsonValue, {"artifacts": artifacts})
    return hashlib.sha256(canonicalize(value)).hexdigest()


def claim_portable_receipt(
    *,
    evidence_root: Path,
    receipt_path: Path,
    receipt_sha256: str,
    scope_sha256: str,
    nonce: str,
    batch_id: str,
    artifact_root: str,
) -> Path:
    """Atomically bind one replay identity to one receipt path."""
    root = evidence_root.resolve(strict=True)
    receipt = _bound_receipt_path(root, receipt_path)
    replay_identity: dict[str, JsonValue] = {
        "scope_sha256": scope_sha256,
        "nonce": nonce,
    }
    claim_id = hashlib.sha256(canonicalize(replay_identity)).hexdigest()
    expected = canonicalize(
        {
            "schema_version": 1,
            **replay_identity,
            "batch_id": batch_id,
            "artifact_root_sha256": artifact_root,
            "receipt_path": receipt.relative_to(root).as_posix(),
            "receipt_sha256": receipt_sha256,
        }
    )
    directory = root / _CLAIM_DIRECTORY
    _secure_claim_directory(directory)
    claim = directory / f"{claim_id}.json"
    try:
        descriptor = os.open(
            claim,
            os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, "O_NOFOLLOW", 0),
            0o600,
        )
    except FileExistsError:
        _validate_existing_claim(claim, expected)
        return claim
    try:
        view = memoryview(expected)
        while view:
            written = os.write(descriptor, view)
            if written <= 0:
                raise PortableReceiptReplayError("portable replay claim write failed")
            view = view[written:]
        os.fsync(descriptor)
    except BaseException:
        os.close(descriptor)
        claim.unlink(missing_ok=True)
        raise
    os.close(descriptor)
    return claim


def _bound_receipt_path(root: Path, supplied: Path) -> Path:
    parent = supplied.parent.resolve(strict=True)
    receipt = parent / supplied.name
    if receipt == root or not receipt.is_relative_to(root):
        raise PortableReceiptReplayError("portable receipt path escapes evidence root")
    if receipt.is_symlink():
        raise PortableReceiptReplayError("portable receipt path is symlinked")
    return receipt


def _secure_claim_directory(directory: Path) -> None:
    try:
        directory.mkdir(mode=0o700)
    except FileExistsError:
        pass
    info = directory.lstat()
    if (
        not stat.S_ISDIR(info.st_mode)
        or stat.S_IMODE(info.st_mode) & 0o077
        or directory.is_symlink()
    ):
        raise PortableReceiptReplayError("portable replay claim permissions differ")


def _validate_existing_claim(claim: Path, expected: bytes) -> None:
    info = claim.lstat()
    if (
        not stat.S_ISREG(info.st_mode)
        or info.st_nlink != 1
        or stat.S_IMODE(info.st_mode) & 0o077
        or claim.is_symlink()
    ):
        raise PortableReceiptReplayError("portable replay claim permissions differ")
    actual = claim.read_bytes()
    parse_strict_object_bytes(actual)
    if actual != expected:
        raise PortableReceiptReplayError("portable receipt identity was replayed")


__all__ = [
    "PortableReceiptReplayError",
    "artifact_root_sha256",
    "claim_portable_receipt",
]
