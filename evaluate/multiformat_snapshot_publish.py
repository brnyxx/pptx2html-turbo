from __future__ import annotations

import errno
import os
import stat
import sys
import tempfile
from collections.abc import Callable
from dataclasses import dataclass
from enum import StrEnum
from pathlib import Path

from evaluate.multiformat_snapshot_filesystem import (
    _clean_directory_fd,
    atomic_rename_noreplace,
)

_DIRECTORY_FLAGS = os.O_RDONLY | getattr(os, "O_DIRECTORY", 0)
_NOFOLLOW_FLAGS = getattr(os, "O_NOFOLLOW", 0)


class SnapshotPublishFailure(StrEnum):
    DESTINATION_EXISTS = "destination-exists"
    LOCKED = "locked"
    PUBLICATION_FAILED = "publication-failed"


class SnapshotPublishError(Exception):
    __slots__ = ("failure", "path")

    def __init__(self, path: Path, failure: SnapshotPublishFailure) -> None:
        self.path = path
        self.failure = failure
        super().__init__(path, failure)

    def __str__(self) -> str:
        return f"snapshot publication failed: {self.failure.value}"


@dataclass(frozen=True, slots=True)
class _InvalidLockNamespaceError(ValueError):
    namespace: str

    def __str__(self) -> str:
        return "lock_namespace must be a lowercase ASCII token"


@dataclass(frozen=True, slots=True)
class _Identity:
    device: int
    inode: int


@dataclass(frozen=True, slots=True)
class _CleanupState:
    parent_descriptor: int
    staging: Path | None
    staging_descriptor: int | None
    staging_identity: _Identity | None
    lock_descriptor: int
    lock: Path
    lock_identity: _Identity
    published: bool


def publish_snapshot(
    destination: Path,
    writer: Callable[[Path], None],
    *,
    lock_namespace: str = "snapshot",
) -> None:
    """Publish a complete directory tree without a readiness marker."""
    _validate_lock_namespace(lock_namespace)
    parent = destination.parent
    parent.mkdir(parents=True, exist_ok=True)
    resolved_parent = parent.resolve(strict=True)
    parent_descriptor = os.open(
        resolved_parent,
        _DIRECTORY_FLAGS | _NOFOLLOW_FLAGS,
    )
    target = resolved_parent / destination.name
    if os.path.lexists(target):
        os.close(parent_descriptor)
        raise SnapshotPublishError(target, SnapshotPublishFailure.DESTINATION_EXISTS)
    lock = resolved_parent / f".{target.name}.{lock_namespace}.lock"
    lock_descriptor, lock_identity = _acquire_lock(lock, parent_descriptor)
    staging: Path | None = None
    staging_descriptor: int | None = None
    staging_identity: _Identity | None = None
    published = False
    try:
        if os.path.lexists(target):
            raise SnapshotPublishError(
                target, SnapshotPublishFailure.DESTINATION_EXISTS
            )
        staging = Path(
            tempfile.mkdtemp(prefix=f".{target.name}.stage-", dir=resolved_parent)
        )
        staging_descriptor = os.open(
            staging,
            _DIRECTORY_FLAGS | _NOFOLLOW_FLAGS,
        )
        staging_identity = _identity(os.fstat(staging_descriptor))
        writer(staging)
        if not _matches(staging, staging_identity, stat.S_ISDIR):
            raise SnapshotPublishError(
                staging, SnapshotPublishFailure.PUBLICATION_FAILED
            )
        try:
            _atomic_rename_noreplace(staging, target, parent_descriptor)
        except OSError as error:
            raise SnapshotPublishError(
                target,
                SnapshotPublishFailure.PUBLICATION_FAILED,
            ) from error
        target_descriptor = os.open(
            target,
            _DIRECTORY_FLAGS | _NOFOLLOW_FLAGS,
        )
        try:
            if _identity(os.fstat(target_descriptor)) != staging_identity:
                raise SnapshotPublishError(
                    target,
                    SnapshotPublishFailure.PUBLICATION_FAILED,
                )
        finally:
            os.close(target_descriptor)
        published = True
    finally:
        active_error = sys.exception()
        errors = _cleanup(
            _CleanupState(
                parent_descriptor,
                staging,
                staging_descriptor,
                staging_identity,
                lock_descriptor,
                lock,
                lock_identity,
                published,
            )
        )
        if active_error is not None:
            for error in errors:
                active_error.add_note(f"snapshot cleanup failed: {error}")
        elif errors:
            publication_error = SnapshotPublishError(
                lock,
                SnapshotPublishFailure.PUBLICATION_FAILED,
            )
            for error in errors:
                publication_error.add_note(f"snapshot cleanup failed: {error}")
            raise publication_error from errors[0]


def _validate_lock_namespace(value: str) -> None:
    if not value or value[0] == "-" or value[-1] == "-":
        raise _InvalidLockNamespaceError(value)
    previous_was_separator = False
    for character in value:
        is_letter = "a" <= character <= "z"
        is_digit = "0" <= character <= "9"
        if character == "-":
            if previous_was_separator:
                raise _InvalidLockNamespaceError(value)
            previous_was_separator = True
        elif is_letter or is_digit:
            previous_was_separator = False
        else:
            raise _InvalidLockNamespaceError(value)


def _acquire_lock(path: Path, parent_descriptor: int) -> tuple[int, _Identity]:
    flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL | _NOFOLLOW_FLAGS
    flags |= getattr(os, "O_CLOEXEC", 0)
    try:
        descriptor = os.open(path.name, flags, 0o600, dir_fd=parent_descriptor)
    except OSError as error:
        raise SnapshotPublishError(path, SnapshotPublishFailure.LOCKED) from error
    return descriptor, _identity(os.fstat(descriptor))


def _identity(value: os.stat_result) -> _Identity:
    return _Identity(value.st_dev, value.st_ino)


def _atomic_rename_noreplace(
    staging: Path,
    target: Path,
    parent_descriptor: int,
) -> None:
    try:
        atomic_rename_noreplace(staging, target, parent_descriptor)
    except FileExistsError as error:
        raise SnapshotPublishError(
            target,
            SnapshotPublishFailure.DESTINATION_EXISTS,
        ) from error


def _cleanup(state: _CleanupState) -> tuple[OSError, ...]:
    errors: list[OSError] = []
    if not state.published and state.staging_descriptor is not None:
        try:
            _remove_owned_directory(
                state.staging,
                state.staging_identity,
                state.staging_descriptor,
                state.parent_descriptor,
            )
        except OSError as error:
            errors.append(error)
    if state.staging_descriptor is not None:
        try:
            os.close(state.staging_descriptor)
        except OSError as error:
            errors.append(error)
    try:
        _unlink_owned_file(state.lock, state.lock_identity, state.parent_descriptor)
    except OSError as error:
        errors.append(error)
    try:
        os.close(state.lock_descriptor)
    except OSError as error:
        errors.append(error)
    try:
        os.close(state.parent_descriptor)
    except OSError as error:
        errors.append(error)
    return tuple(errors)


def _matches(
    path: Path | None,
    identity: _Identity | None,
    expected_mode: Callable[[int], bool],
) -> bool:
    if path is None or identity is None:
        return False
    try:
        value = path.lstat()
    except FileNotFoundError:
        return False
    return expected_mode(value.st_mode) and _identity(value) == identity


def _remove_owned_directory(
    path: Path | None,
    identity: _Identity | None,
    descriptor: int,
    parent_descriptor: int,
) -> None:
    if not _matches(path, identity, stat.S_ISDIR):
        _clean_directory_fd(descriptor)
        raise OSError(errno.ESTALE, "staging path ownership changed")
    _clean_directory_fd(descriptor)
    if not _matches(path, identity, stat.S_ISDIR):
        raise OSError(errno.ESTALE, "staging path ownership changed")
    assert path is not None
    os.rmdir(path.name, dir_fd=parent_descriptor)


def _unlink_owned_file(
    path: Path,
    identity: _Identity,
    parent_descriptor: int,
) -> None:
    if not _matches(path, identity, stat.S_ISREG):
        raise OSError(errno.ESTALE, "lock path ownership changed")
    descriptor = os.open(
        path.name,
        os.O_WRONLY | _NOFOLLOW_FLAGS,
        dir_fd=parent_descriptor,
    )
    try:
        if _identity(os.fstat(descriptor)) != identity:
            raise OSError(errno.ESTALE, "lock inode changed")
        os.unlink(path.name, dir_fd=parent_descriptor)
    finally:
        os.close(descriptor)
