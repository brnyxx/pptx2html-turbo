"""Deterministic identity for an immutable READY corpus tree."""

from __future__ import annotations

import hashlib
import os
import stat
from dataclasses import dataclass
from pathlib import Path
from typing import Final

from evaluate.jcs import JcsError, canonicalize
from evaluate.multiformat_schema import JsonValue

_ASSEMBLY_MANIFEST_NAME: Final = "assembly-manifest.json"
_HASH_CHUNK_SIZE: Final = 1024 * 1024
_NO_FOLLOW: Final = getattr(os, "O_NOFOLLOW", 0)


@dataclass(frozen=True, slots=True)
class TreeIdentityError(ValueError):
    """A corpus tree contains an unsafe or unsupported filesystem entry."""

    reason: str

    def __post_init__(self) -> None:
        ValueError.__init__(self, self.reason)


@dataclass(frozen=True, slots=True)
class TreeIdentity:
    """The canonical digest and physical byte counts for a corpus tree."""

    sha256: str
    entry_count: int
    total_bytes: int

    @property
    def files(self) -> int:
        """Return the number of regular files bound by this identity."""
        return self.entry_count

    @property
    def bytes(self) -> int:
        """Return the total size of regular files bound by this identity."""
        return self.total_bytes


@dataclass(frozen=True, slots=True)
class _FileRecord:
    path: str
    sha256: str
    size: int


def tree_identity(root: Path) -> TreeIdentity:
    """Hash every safe regular file beneath ``root`` using canonical framing."""
    root_stat = _root_stat(root)
    if not stat.S_ISDIR(root_stat.st_mode):
        raise TreeIdentityError(reason=f"tree root is not a directory: {root}")

    records: list[_FileRecord] = []
    inodes: set[tuple[int, int]] = set()
    directories = [root]
    while directories:
        directory = directories.pop()
        for entry in _sorted_entries(directory):
            relative_path = _safe_relative_path(root, Path(entry.path))
            entry_stat = _entry_stat(entry, relative_path)
            mode = entry_stat.st_mode
            if stat.S_ISLNK(mode):
                raise TreeIdentityError(
                    reason=f"symlink is not allowed: {relative_path}"
                )
            if stat.S_ISDIR(mode):
                directories.append(Path(entry.path))
                continue
            if not stat.S_ISREG(mode):
                raise TreeIdentityError(
                    reason=f"special file is not allowed: {relative_path}",
                )
            inode = (entry_stat.st_dev, entry_stat.st_ino)
            if entry_stat.st_nlink != 1:
                raise TreeIdentityError(
                    reason=f"hard link is not allowed: {relative_path}"
                )
            if inode in inodes:
                raise TreeIdentityError(
                    reason=f"duplicate inode is not allowed: {relative_path}"
                )
            inodes.add(inode)
            if relative_path == _ASSEMBLY_MANIFEST_NAME:
                continue
            records.append(
                _FileRecord(
                    path=relative_path,
                    sha256=_hash_file(Path(entry.path), entry_stat, relative_path),
                    size=entry_stat.st_size,
                ),
            )

    ordered = sorted(records, key=lambda record: record.path.encode("utf-8"))
    files: list[JsonValue] = [
        {"path": record.path, "sha256": record.sha256, "size": record.size}
        for record in ordered
    ]
    payload: JsonValue = {"schema_version": 1, "files": files}
    try:
        canonical = canonicalize(payload)
    except JcsError as error:
        raise TreeIdentityError(
            reason="tree identity canonicalization failed"
        ) from error
    return TreeIdentity(
        sha256=hashlib.sha256(canonical).hexdigest(),
        entry_count=len(ordered),
        total_bytes=sum(record.size for record in ordered),
    )


def _root_stat(root: Path) -> os.stat_result:
    try:
        return root.lstat()
    except OSError as error:
        raise TreeIdentityError(reason=f"cannot inspect tree root: {root}") from error


def _sorted_entries(directory: Path) -> list[os.DirEntry[str]]:
    try:
        with os.scandir(directory) as entries:
            return sorted(entries, key=lambda entry: os.fsencode(entry.name))
    except OSError as error:
        raise TreeIdentityError(
            reason=f"cannot enumerate tree directory: {directory}"
        ) from error


def _safe_relative_path(root: Path, path: Path) -> str:
    try:
        relative = path.relative_to(root)
        value = relative.as_posix()
        _ = value.encode("utf-8")
    except (UnicodeError, ValueError) as error:
        raise TreeIdentityError(reason=f"unsafe relative path: {path}") from error
    if (
        not value
        or relative.is_absolute()
        or any(part in {"", ".", ".."} for part in relative.parts)
    ):
        raise TreeIdentityError(reason=f"unsafe relative path: {value}")
    return value


def _entry_stat(entry: os.DirEntry[str], relative_path: str) -> os.stat_result:
    try:
        return entry.stat(follow_symlinks=False)
    except OSError as error:
        raise TreeIdentityError(
            reason=f"cannot inspect tree entry: {relative_path}"
        ) from error


def _hash_file(
    path: Path,
    expected: os.stat_result,
    relative_path: str,
) -> str:
    try:
        descriptor = os.open(path, os.O_RDONLY | _NO_FOLLOW)
    except OSError as error:
        raise TreeIdentityError(
            reason=f"cannot open tree entry: {relative_path}"
        ) from error
    try:
        with os.fdopen(descriptor, "rb") as source:
            opened = os.fstat(source.fileno())
            if not _same_file_identity(expected, opened):
                raise TreeIdentityError(
                    reason=f"tree entry changed during hashing: {relative_path}",
                )
            digest = hashlib.sha256()
            while chunk := source.read(_HASH_CHUNK_SIZE):
                digest.update(chunk)
            finished = os.fstat(source.fileno())
            if not _same_file_identity(expected, finished):
                raise TreeIdentityError(
                    reason=f"tree entry changed during hashing: {relative_path}",
                )
            return digest.hexdigest()
    except OSError as error:
        raise TreeIdentityError(
            reason=f"cannot read tree entry: {relative_path}"
        ) from error


def _same_file_identity(first: os.stat_result, second: os.stat_result) -> bool:
    return (
        first.st_dev,
        first.st_ino,
        first.st_nlink,
        first.st_size,
    ) == (
        second.st_dev,
        second.st_ino,
        second.st_nlink,
        second.st_size,
    )
