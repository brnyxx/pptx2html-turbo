from __future__ import annotations

import os
import shutil
import stat
import sys
import tempfile
from collections.abc import Callable
from dataclasses import dataclass
from enum import StrEnum
from pathlib import Path


class SnapshotPublishFailure(StrEnum):
    DESTINATION_EXISTS = "destination-exists"
    LOCKED = "locked"
    PUBLICATION_FAILED = "publication-failed"


@dataclass(frozen=True, slots=True)
class SnapshotPublishError(Exception):
    path: Path
    failure: SnapshotPublishFailure

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
    published: bool
    staging: Path | None
    staging_identity: _Identity | None
    lock_descriptor: int
    lock: Path
    lock_identity: _Identity


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
    target = resolved_parent / destination.name
    if os.path.lexists(target):
        raise SnapshotPublishError(
            target,
            SnapshotPublishFailure.DESTINATION_EXISTS,
        )
    lock = resolved_parent / f".{target.name}.{lock_namespace}.lock"
    lock_descriptor, lock_identity = _acquire_lock(lock)
    staging: Path | None = None
    staging_identity: _Identity | None = None
    published = False
    try:
        if os.path.lexists(target):
            raise SnapshotPublishError(
                target,
                SnapshotPublishFailure.DESTINATION_EXISTS,
            )
        staging = Path(
            tempfile.mkdtemp(
                prefix=f".{target.name}.stage-",
                dir=resolved_parent,
            )
        )
        staging_identity = _identity(staging.stat())
        writer(staging)
        if os.path.lexists(target):
            raise SnapshotPublishError(
                target,
                SnapshotPublishFailure.DESTINATION_EXISTS,
            )
        if not _matches(staging, staging_identity, stat.S_ISDIR):
            raise SnapshotPublishError(
                staging,
                SnapshotPublishFailure.PUBLICATION_FAILED,
            )
        try:
            os.rename(staging, target)
        except OSError as error:
            raise SnapshotPublishError(
                target,
                SnapshotPublishFailure.PUBLICATION_FAILED,
            ) from error
        published = True
    finally:
        active_error = sys.exception()
        cleanup_error = _cleanup(
            _CleanupState(
                published,
                staging,
                staging_identity,
                lock_descriptor,
                lock,
                lock_identity,
            )
        )
        if cleanup_error is not None:
            if active_error is not None:
                active_error.add_note(f"snapshot cleanup failed: {cleanup_error}")
            else:
                raise SnapshotPublishError(
                    lock,
                    SnapshotPublishFailure.PUBLICATION_FAILED,
                ) from cleanup_error


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


def _acquire_lock(path: Path) -> tuple[int, _Identity]:
    flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL
    flags |= getattr(os, "O_CLOEXEC", 0) | getattr(os, "O_NOFOLLOW", 0)
    try:
        descriptor = os.open(path, flags, 0o600)
    except OSError as error:
        raise SnapshotPublishError(
            path,
            SnapshotPublishFailure.LOCKED,
        ) from error
    return descriptor, _identity(os.fstat(descriptor))


def _identity(value: os.stat_result) -> _Identity:
    return _Identity(value.st_dev, value.st_ino)


def _cleanup(state: _CleanupState) -> OSError | None:
    first_error: OSError | None = None
    if (
        not state.published
        and state.staging is not None
        and state.staging_identity is not None
    ):
        try:
            _remove_owned_directory(state.staging, state.staging_identity)
        except OSError as error:
            first_error = error
    try:
        os.close(state.lock_descriptor)
    except OSError as error:
        first_error = first_error or error
    try:
        _unlink_owned_file(state.lock, state.lock_identity)
    except OSError as error:
        first_error = first_error or error
    return first_error


def _matches(
    path: Path,
    identity: _Identity,
    expected_mode: Callable[[int], bool],
) -> bool:
    try:
        value = path.lstat()
    except FileNotFoundError:
        return False
    return (
        expected_mode(value.st_mode)
        and value.st_dev == identity.device
        and value.st_ino == identity.inode
    )


def _remove_owned_directory(path: Path, identity: _Identity) -> None:
    if _matches(path, identity, stat.S_ISDIR):
        shutil.rmtree(path)


def _unlink_owned_file(path: Path, identity: _Identity) -> None:
    if _matches(path, identity, stat.S_ISREG):
        path.unlink()
