"""Stable filesystem validation for portable receipts."""

from __future__ import annotations

import hashlib
import os
import stat
from dataclasses import dataclass
from pathlib import Path
from typing import TypeAlias

from evaluate.multiformat_evidence import resolve_evidence_path
from evaluate.multiformat_schema import (
    JsonValue,
    integer_value,
    sha256_value,
    string_value,
)

JsonObject: TypeAlias = dict[str, JsonValue]


class ReceiptValidationError(ValueError):
    """A receipt record violates the portable schema."""


@dataclass(frozen=True, slots=True)
class StableFileIdentity:
    path: str
    sha256: str
    size: int
    role: str
    device: int
    inode: int
    mtime_ns: int
    ctime_ns: int


def validate_artifact_records(artifacts: list[JsonObject]) -> None:
    if not artifacts:
        raise ReceiptValidationError("portable receipt artifacts are empty")
    paths: set[str] = set()
    for artifact in artifacts:
        require_exact_keys(artifact, {"path", "sha256", "size", "role"}, "artifact")
        path = string_value(artifact, "path")
        sha256_value(artifact, "sha256")
        if integer_value(artifact, "size") < 0:
            raise ReceiptValidationError("portable receipt artifact size is invalid")
        string_value(artifact, "role")
        if path in paths:
            raise ReceiptValidationError("portable receipt artifact path is duplicated")
        paths.add(path)
    if artifacts != sorted(artifacts, key=lambda item: string_value(item, "path")):
        raise ReceiptValidationError("portable receipt artifacts are not ordered")


def verify_artifacts(
    artifacts: list[JsonObject],
    evidence_root: Path,
) -> tuple[StableFileIdentity, ...]:
    validate_artifact_records(artifacts)
    identities = tuple(
        verify_stable_file(
            evidence_root,
            string_value(artifact, "path"),
            sha256_value(artifact, "sha256"),
            integer_value(artifact, "size"),
            string_value(artifact, "role"),
        )
        for artifact in artifacts
    )
    canonical_paths = {identity.path for identity in identities}
    inodes = {(identity.device, identity.inode) for identity in identities}
    if len(canonical_paths) != len(identities):
        raise ReceiptValidationError("portable receipt canonical path is duplicated")
    if len(inodes) != len(identities):
        raise ReceiptValidationError("portable receipt artifact inode alias detected")
    return identities


def reject_identity_aliases(
    identity_sets: tuple[tuple[StableFileIdentity, ...], ...],
) -> None:
    """Reject canonical-path or inode reuse across all supplied identity sets."""
    identities = tuple(identity for values in identity_sets for identity in values)
    paths = {identity.path for identity in identities}
    inodes = {(identity.device, identity.inode) for identity in identities}
    if len(paths) != len(identities) or len(inodes) != len(identities):
        raise ReceiptValidationError(
            "portable receipt cross-set identity alias detected"
        )


def verify_stable_file(
    evidence_root: Path,
    relative_path: str,
    expected_sha256: str,
    expected_size: int | None,
    role: str,
) -> StableFileIdentity:
    """Hash one no-follow regular file and prove its path identity stayed stable."""
    if evidence_root.is_symlink():
        raise ReceiptValidationError("portable receipt evidence root is a symlink")
    root = evidence_root.resolve(strict=True)
    path = resolve_evidence_path(root, relative_path)
    flags = os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0)
    first_digest, first_before, first_after = _descriptor_pass(path, flags)
    first_path = path.lstat()
    _require_stable_pass(first_before, first_after, first_path)
    second_digest, second_before, second_after = _descriptor_pass(path, flags)
    final_path = path.lstat()
    identities = (
        _stat_identity(first_before),
        _stat_identity(first_after),
        _stat_identity(first_path),
        _stat_identity(second_before),
        _stat_identity(second_after),
        _stat_identity(final_path),
    )
    if len(set(identities)) != 1 or first_digest != second_digest:
        raise ReceiptValidationError("portable receipt file changed across hash passes")
    if first_before.st_nlink != 1:
        raise ReceiptValidationError(
            "portable receipt artifact has an external hardlink alias"
        )
    if first_digest != expected_sha256 or (
        expected_size is not None and first_before.st_size != expected_size
    ):
        raise ReceiptValidationError("portable receipt artifact binding differs")
    return StableFileIdentity(
        path=path.relative_to(root).as_posix(),
        sha256=first_digest,
        size=first_before.st_size,
        role=role,
        device=first_before.st_dev,
        inode=first_before.st_ino,
        mtime_ns=first_before.st_mtime_ns,
        ctime_ns=first_before.st_ctime_ns,
    )


def object_array(values: JsonObject, field: str) -> list[JsonObject]:
    value = values.get(field)
    if not isinstance(value, list):
        raise ReceiptValidationError(f"portable receipt {field} is not an array")
    result: list[JsonObject] = []
    for item in value:
        if not isinstance(item, dict):
            raise ReceiptValidationError(f"portable receipt {field} item is invalid")
        result.append(item)
    return result


def hex_bytes(values: JsonObject, field: str, size: int) -> bytes:
    value = string_value(values, field)
    if len(value) != size * 2 or any(
        character not in "0123456789abcdef" for character in value
    ):
        raise ReceiptValidationError(f"portable receipt {field} is malformed")
    return bytes.fromhex(value)


def require_exact_keys(values: JsonObject, expected: set[str], label: str) -> None:
    if set(values) != expected:
        raise ReceiptValidationError(f"portable receipt {label} fields differ")


def _descriptor_pass(
    path: Path,
    flags: int,
) -> tuple[str, os.stat_result, os.stat_result]:
    descriptor = os.open(path, flags)
    try:
        before = os.fstat(descriptor)
        if not stat.S_ISREG(before.st_mode):
            raise ReceiptValidationError("portable receipt artifact is not regular")
        digest = _hash_descriptor(descriptor)
        _after_file_hash(path)
        after = os.fstat(descriptor)
        return digest, before, after
    finally:
        os.close(descriptor)


def _require_stable_pass(
    before: os.stat_result,
    after: os.stat_result,
    path_stat: os.stat_result,
) -> None:
    if (
        not stat.S_ISREG(path_stat.st_mode)
        or len(
            {_stat_identity(before), _stat_identity(after), _stat_identity(path_stat)}
        )
        != 1
    ):
        raise ReceiptValidationError("portable receipt file changed while hashing")


def _stat_identity(value: os.stat_result) -> tuple[int, int, int, int, int]:
    return (
        value.st_dev,
        value.st_ino,
        value.st_size,
        value.st_mtime_ns,
        value.st_ctime_ns,
    )


def _hash_descriptor(descriptor: int) -> str:
    digest = hashlib.sha256()
    os.lseek(descriptor, 0, os.SEEK_SET)
    while chunk := os.read(descriptor, 1024 * 1024):
        digest.update(chunk)
    return digest.hexdigest()


def _after_file_hash(_path: Path) -> None:
    """Deterministic race-test seam; production performs no action."""
