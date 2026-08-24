"""Stable descriptor operations for typed corpus source validation."""

from __future__ import annotations

import hashlib
import os
import stat
from collections.abc import Generator
from contextlib import contextmanager
from dataclasses import dataclass
from pathlib import Path

from evaluate.multiformat_corpus_types import CorpusError
from evaluate.multiformat_package_validation import MAX_SOURCE_BYTES
from evaluate.multiformat_ready_tree_io import fd_scope
from evaluate.multiformat_ready_tree_types import TreeIdentityError

FileIdentity = tuple[int, int, int, int, int, int]


@dataclass(frozen=True, slots=True)
class SourceDescriptor:
    descriptor: int
    digest: str
    identity: FileIdentity


@contextmanager
def stable_source_descriptor(
    path: Path,
    relative_path: str,
) -> Generator[SourceDescriptor, None, None]:
    """Hash one owned descriptor and verify its path identity on exit."""
    try:
        no_follow = os.O_NOFOLLOW
    except AttributeError as error:
        raise CorpusError("source.path", relative_path) from error
    try:
        path_before = path.lstat()
    except OSError as error:
        raise CorpusError("source.path", relative_path) from error
    _require_regular(path_before, relative_path)
    initial_identity = file_identity(path_before)
    try:
        with fd_scope(path, os.O_RDONLY | no_follow, None) as descriptor:
            opened = _stat_descriptor(descriptor, relative_path)
            if not _same_identity(path_before, opened):
                raise CorpusError("source.changed", relative_path)
            digest, before, after = _hash_descriptor(descriptor, relative_path)
            if not all(_same_identity(path_before, value) for value in (before, after)):
                raise CorpusError("source.changed", relative_path)
            yield SourceDescriptor(descriptor, digest, initial_identity)
            final_descriptor = _stat_descriptor(descriptor, relative_path)
            final_path = _stat_path(path, relative_path)
            if not _same_identity(
                path_before,
                final_descriptor,
            ) or not _same_identity(path_before, final_path):
                raise CorpusError("source.changed", relative_path)
    except TreeIdentityError as error:
        raise CorpusError("source.path", relative_path) from error


def descriptor_path(descriptor: int) -> Path:
    """Return a path that reopens the already-owned descriptor, not its name."""
    return Path(f"/dev/fd/{descriptor}")


def rewind_descriptor(descriptor: int, relative_path: str) -> None:
    try:
        _ = os.lseek(descriptor, 0, os.SEEK_SET)
    except OSError as error:
        raise CorpusError("source.read", relative_path) from error


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
        raise CorpusError("source.read", relative_path) from error
    return digest.hexdigest(), before, after


def _stat_descriptor(descriptor: int, relative_path: str) -> os.stat_result:
    try:
        value = os.fstat(descriptor)
    except OSError as error:
        raise CorpusError("source.path", relative_path) from error
    _require_regular(value, relative_path)
    return value


def _stat_path(path: Path, relative_path: str) -> os.stat_result:
    try:
        value = path.lstat()
    except OSError as error:
        raise CorpusError("source.changed", relative_path) from error
    _require_regular(value, relative_path)
    return value


def _require_regular(value: os.stat_result, relative_path: str) -> None:
    if not stat.S_ISREG(value.st_mode):
        raise CorpusError("source.path", relative_path)
    if value.st_nlink != 1:
        raise CorpusError("source.link", relative_path)
    if not 0 < value.st_size <= MAX_SOURCE_BYTES:
        raise CorpusError("source.size", relative_path)


def file_identity(value: os.stat_result) -> FileIdentity:
    return (
        value.st_dev,
        value.st_ino,
        value.st_size,
        value.st_mtime_ns,
        value.st_ctime_ns,
        value.st_nlink,
    )


def _same_identity(first: os.stat_result, second: os.stat_result) -> bool:
    return first[:7] == second[:7] and (
        first.st_mtime_ns,
        first.st_ctime_ns,
    ) == (second.st_mtime_ns, second.st_ctime_ns)
