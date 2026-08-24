"""Stable descriptor bindings for public-pool files and manifests."""

from __future__ import annotations

import hashlib
import os
import stat
from dataclasses import dataclass
from pathlib import Path

from evaluate.multiformat_corpus_source_fs import FileIdentity, file_identity
from evaluate.multiformat_package_validation import MAX_SOURCE_BYTES
from evaluate.multiformat_public_pool_types import PublicPoolError
from evaluate.multiformat_ready_tree_io import fd_scope
from evaluate.multiformat_ready_tree_types import TreeIdentityError
from evaluate.multiformat_strict_json import MAX_JSON_BYTES


@dataclass(frozen=True, slots=True)
class ExpectedFileBinding:
    relative_path: str
    sha256: str
    identity: FileIdentity | None


def read_manifest_bytes(
    root: Path,
    manifest: Path,
    expected_identity: FileIdentity,
) -> tuple[bytes, ExpectedFileBinding]:
    try:
        no_follow = os.O_NOFOLLOW
    except AttributeError as error:
        raise PublicPoolError(
            "public pool manifest requires no-follow opens"
        ) from error
    path_before = _stat_regular(manifest, MAX_JSON_BYTES, "manifest")
    if file_identity(path_before) != expected_identity:
        raise PublicPoolError("public pool manifest changed before reading")
    try:
        with fd_scope(manifest, os.O_RDONLY | no_follow, None) as descriptor:
            opened = _stat_regular_descriptor(descriptor, MAX_JSON_BYTES, "manifest")
            if file_identity(opened) != expected_identity:
                raise PublicPoolError("public pool manifest was replaced")
            value = _read_descriptor(descriptor, opened.st_size, "manifest")
            after = _stat_regular_descriptor(descriptor, MAX_JSON_BYTES, "manifest")
            final_path = _stat_regular(manifest, MAX_JSON_BYTES, "manifest")
            if (
                file_identity(after) != expected_identity
                or file_identity(final_path) != expected_identity
            ):
                raise PublicPoolError("public pool manifest changed while reading")
    except TreeIdentityError as error:
        raise PublicPoolError("public pool manifest open failed") from error
    binding = ExpectedFileBinding(
        manifest.relative_to(root).as_posix(),
        hashlib.sha256(value).hexdigest(),
        expected_identity,
    )
    return value, binding


def verify_file_binding(
    parent_descriptor: int,
    name: str,
    relative_path: str,
    binding: ExpectedFileBinding,
    no_follow: int,
) -> None:
    try:
        with fd_scope(
            name,
            os.O_RDONLY | no_follow,
            parent_descriptor,
        ) as descriptor:
            opened = _stat_regular_descriptor(
                descriptor,
                MAX_SOURCE_BYTES,
                relative_path,
            )
            digest, before, after = _hash_descriptor(descriptor, relative_path)
            if (
                binding.identity is not None
                and file_identity(opened) != binding.identity
            ):
                raise PublicPoolError(
                    f"public pool file identity differs: {relative_path}"
                )
            final_path = _stat_entry(parent_descriptor, name, relative_path)
            if not _same_file_identity(before, after) or not _same_file_identity(
                before, final_path
            ):
                raise PublicPoolError(f"public pool file changed: {relative_path}")
            if digest != binding.sha256:
                raise PublicPoolError(f"public pool file bytes differ: {relative_path}")
    except TreeIdentityError as error:
        raise PublicPoolError(
            f"public pool file open failed: {relative_path}"
        ) from error


def _read_descriptor(
    descriptor: int,
    size: int,
    relative_path: str,
) -> bytes:
    chunks: list[bytes] = []
    remaining = size
    try:
        while remaining:
            chunk = os.read(descriptor, min(1024 * 1024, remaining))
            if not chunk:
                raise PublicPoolError(f"public pool file truncated: {relative_path}")
            chunks.append(chunk)
            remaining -= len(chunk)
    except OSError as error:
        raise PublicPoolError(
            f"public pool file read failed: {relative_path}"
        ) from error
    return b"".join(chunks)


def _hash_descriptor(
    descriptor: int,
    relative_path: str,
) -> tuple[str, os.stat_result, os.stat_result]:
    try:
        _ = os.lseek(descriptor, 0, os.SEEK_SET)
        before = os.fstat(descriptor)
        digest = hashlib.sha256()
        while chunk := os.read(descriptor, 1024 * 1024):
            digest.update(chunk)
        after = os.fstat(descriptor)
    except OSError as error:
        raise PublicPoolError(
            f"public pool file read failed: {relative_path}"
        ) from error
    return digest.hexdigest(), before, after


def _stat_regular(path: Path, limit: int, role: str) -> os.stat_result:
    try:
        value = path.lstat()
    except OSError as error:
        raise PublicPoolError(f"public pool {role} is unavailable") from error
    return _require_regular(value, limit, role)


def _stat_regular_descriptor(
    descriptor: int,
    limit: int,
    role: str,
) -> os.stat_result:
    try:
        value = os.fstat(descriptor)
    except OSError as error:
        raise PublicPoolError(f"public pool {role} is unavailable") from error
    return _require_regular(value, limit, role)


def _stat_entry(parent: int, name: str, relative_path: str) -> os.stat_result:
    try:
        value = os.stat(name, dir_fd=parent, follow_symlinks=False)
    except OSError as error:
        raise PublicPoolError(
            f"public pool file disappeared: {relative_path}"
        ) from error
    return _require_regular(value, MAX_SOURCE_BYTES, relative_path)


def _require_regular(
    value: os.stat_result,
    limit: int,
    role: str,
) -> os.stat_result:
    if not stat.S_ISREG(value.st_mode):
        raise PublicPoolError(f"public pool {role} is not regular")
    if value.st_nlink != 1:
        raise PublicPoolError(f"public pool {role} is a hard link")
    if not 0 < value.st_size <= limit:
        raise PublicPoolError(f"public pool {role} size is invalid")
    return value


def _same_file_identity(first: os.stat_result, second: os.stat_result) -> bool:
    return file_identity(first) == file_identity(second)
