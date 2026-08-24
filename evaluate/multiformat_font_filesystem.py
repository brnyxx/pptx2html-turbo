from __future__ import annotations

import hashlib
import os
import stat
from dataclasses import dataclass
from pathlib import Path

_DIRECTORY_FLAGS = os.O_RDONLY | getattr(os, "O_DIRECTORY", 0)
_NOFOLLOW_FLAGS = getattr(os, "O_NOFOLLOW", 0)
_CHUNK_SIZE = 1024 * 1024


@dataclass(frozen=True, slots=True)
class DiscoveredFont:
    path: Path
    digest: str
    suffix: str
    identity: tuple[int, int]


@dataclass(frozen=True, slots=True)
class StableFile:
    data: bytes
    signature: tuple[int, int, int, int, int]
    digest: str


def discover_font_root(
    root: Path,
    identities: set[tuple[int, int]],
    digests: set[str],
) -> tuple[DiscoveredFont, ...]:
    information = root.lstat()
    if not stat.S_ISDIR(information.st_mode):
        raise OSError("font root is not a directory")
    return _walk_font_root(
        root,
        (information.st_dev, information.st_ino),
        identities,
        digests,
    )


def copy_font_file(
    source: Path,
    expected_identity: tuple[int, int],
    expected_digest: str,
    destination: Path,
) -> None:
    source_descriptor = os.open(
        source,
        os.O_RDONLY | _NOFOLLOW_FLAGS,
    )
    destination_descriptor: int | None = None
    try:
        before = _regular_signature(os.fstat(source_descriptor))
        if before[:2] != expected_identity:
            raise OSError("font source identity changed")
        destination_descriptor = os.open(
            destination,
            os.O_WRONLY | os.O_CREAT | os.O_EXCL | _NOFOLLOW_FLAGS,
            0o644,
        )
        digest = hashlib.sha256()
        while chunk := os.read(source_descriptor, _CHUNK_SIZE):
            digest.update(chunk)
            _write_all(destination_descriptor, chunk)
        after = _regular_signature(os.fstat(source_descriptor))
        if before != after or digest.hexdigest() != expected_digest:
            raise OSError("font source changed during copy")
    finally:
        os.close(source_descriptor)
        if destination_descriptor is not None:
            os.close(destination_descriptor)


def read_stable_file(path: Path) -> StableFile:
    descriptor = os.open(path, os.O_RDONLY | _NOFOLLOW_FLAGS)
    try:
        before = _regular_signature(os.fstat(descriptor))
        data = _read_all(descriptor)
        after = _regular_signature(os.fstat(descriptor))
        if before != after:
            raise OSError("file changed while reading")
        return StableFile(data, after, hashlib.sha256(data).hexdigest())
    finally:
        os.close(descriptor)


def revalidate_file(path: Path, expected: StableFile) -> None:
    actual = read_stable_file(path)
    if actual != expected:
        raise OSError("file changed after validation")


def _walk_font_root(
    root: Path,
    expected_identity: tuple[int, int],
    identities: set[tuple[int, int]],
    digests: set[str],
) -> tuple[DiscoveredFont, ...]:
    descriptor = os.open(root, _DIRECTORY_FLAGS | _NOFOLLOW_FLAGS)
    try:
        if _identity(os.fstat(descriptor)) != expected_identity:
            raise OSError("font directory changed during discovery")
        with os.scandir(descriptor) as entries:
            children = sorted(entries, key=lambda entry: entry.name)
        discovered: list[DiscoveredFont] = []
        for entry in children:
            information = entry.stat(follow_symlinks=False)
            path = root / entry.name
            if stat.S_ISLNK(information.st_mode):
                raise OSError("font roots may not contain links")
            if stat.S_ISDIR(information.st_mode):
                discovered.extend(
                    _walk_font_root(
                        path,
                        _identity(information),
                        identities,
                        digests,
                    )
                )
                continue
            if not stat.S_ISREG(information.st_mode) or information.st_nlink != 1:
                raise OSError("font root contains an invalid file")
            suffix = path.suffix.lower()
            if suffix not in {".ttf", ".otf"}:
                raise OSError("unsupported font suffix")
            resolved = path.resolve(strict=True)
            if not resolved.is_relative_to(root):
                raise OSError("font escapes its root")
            identity = _identity(information)
            if identity in identities:
                raise OSError("font inode is repeated")
            digest = _hash_entry(descriptor, entry.name, information)
            if digest in digests:
                raise OSError("font digest is repeated")
            identities.add(identity)
            digests.add(digest)
            discovered.append(DiscoveredFont(resolved, digest, suffix, identity))
        return tuple(discovered)
    finally:
        os.close(descriptor)


def _hash_entry(
    parent_descriptor: int,
    name: str,
    expected: os.stat_result,
) -> str:
    descriptor = os.open(name, os.O_RDONLY | _NOFOLLOW_FLAGS, dir_fd=parent_descriptor)
    try:
        before = _regular_signature(os.fstat(descriptor))
        if before != _regular_signature(expected):
            raise OSError("font entry changed during discovery")
        digest = hashlib.sha256()
        while chunk := os.read(descriptor, _CHUNK_SIZE):
            digest.update(chunk)
        if before != _regular_signature(os.fstat(descriptor)):
            raise OSError("font entry changed during discovery")
        return digest.hexdigest()
    finally:
        os.close(descriptor)


def _regular_signature(value: os.stat_result) -> tuple[int, int, int, int, int]:
    if not stat.S_ISREG(value.st_mode) or value.st_nlink != 1:
        raise OSError("expected a standalone regular file")
    return (
        value.st_dev,
        value.st_ino,
        value.st_size,
        value.st_mtime_ns,
        value.st_ctime_ns,
    )


def _identity(value: os.stat_result) -> tuple[int, int]:
    return value.st_dev, value.st_ino


def _read_all(descriptor: int) -> bytes:
    chunks: list[bytes] = []
    while chunk := os.read(descriptor, _CHUNK_SIZE):
        chunks.append(chunk)
    return b"".join(chunks)


def _write_all(descriptor: int, data: bytes) -> None:
    view = memoryview(data)
    while view:
        view = view[os.write(descriptor, view) :]
