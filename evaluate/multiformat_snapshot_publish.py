from __future__ import annotations

import errno
import os
import stat
import sys
import tempfile
from collections.abc import Callable
from enum import StrEnum
from pathlib import Path

from evaluate.multiformat_snapshot_cleanup import remove_owned_staging
from evaluate.multiformat_snapshot_filesystem import (
    atomic_rename_noreplace,
    directory_identity,
    open_owned_directory,
    unlink_owned_file,
    valid_lock_namespace,
    verify_directory_identity,
)
from evaluate.multiformat_snapshot_lock import acquire_lock
from evaluate.multiformat_snapshot_publish_types import (
    CleanupState as _CleanupState,
    Identity as _Identity,
    InvalidLockNamespaceError as _InvalidLockNamespaceError,
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


def publish_snapshot(
    destination: Path,
    writer: Callable[[Path], None],
    *,
    lock_namespace: str = "snapshot",
    before_publish: Callable[[], None] | None = None,
) -> None:
    """Publish a complete directory tree without a readiness marker."""
    if not valid_lock_namespace(lock_namespace):
        raise _InvalidLockNamespaceError(lock_namespace)
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
    try:
        lock_descriptor, lock_identity = _acquire_lock(lock, parent_descriptor)
    except SnapshotPublishError as error:
        try:
            os.close(parent_descriptor)
        except OSError as cleanup_error:
            error.add_note(f"snapshot cleanup failed: {cleanup_error}")
        raise
    staging: Path | None = None
    staging_descriptor: int | None = None
    staging_identity: _Identity | None = None
    renamed = False
    published = False
    try:
        if os.path.lexists(target):
            raise SnapshotPublishError(
                target, SnapshotPublishFailure.DESTINATION_EXISTS
            )
        staging = Path(
            tempfile.mkdtemp(prefix=f".{target.name}.stage-", dir=resolved_parent)
        )
        try:
            created_identity = directory_identity(parent_descriptor, staging.name)
            staging_identity = _Identity(*created_identity)
            staging_descriptor = open_owned_directory(staging, created_identity)
        except OSError as error:
            if staging_identity is None:
                try:
                    staging_identity = _Identity(
                        *directory_identity(parent_descriptor, staging.name)
                    )
                except OSError as cleanup_error:
                    error.add_note(f"snapshot cleanup failed: {cleanup_error}")
            raise SnapshotPublishError(
                staging,
                SnapshotPublishFailure.PUBLICATION_FAILED,
            ) from error
        writer(staging)
        if not _matches(staging, staging_identity, stat.S_ISDIR):
            raise SnapshotPublishError(
                staging, SnapshotPublishFailure.PUBLICATION_FAILED
            )
        if before_publish is not None:
            before_publish()
        try:
            _atomic_rename_noreplace(staging, target, parent_descriptor)
            renamed = True
        except OSError as error:
            raise SnapshotPublishError(
                target,
                SnapshotPublishFailure.PUBLICATION_FAILED,
            ) from error
        if staging_identity is None:
            # The staging identity is captured before the rename, so a missing
            # value here means the publication invariant was broken.
            raise SnapshotPublishError(
                target,
                SnapshotPublishFailure.PUBLICATION_FAILED,
            )
        try:
            verify_directory_identity(
                target,
                (staging_identity.device, staging_identity.inode),
            )
        except OSError as error:
            raise SnapshotPublishError(
                target,
                SnapshotPublishFailure.PUBLICATION_FAILED,
            ) from error
        published = True
    finally:
        active_error = sys.exception()
        errors = _cleanup(
            _CleanupState(
                parent_descriptor,
                staging,
                target,
                staging_descriptor,
                staging_identity,
                renamed,
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


def _acquire_lock(path: Path, parent_descriptor: int) -> tuple[int, _Identity]:
    try:
        descriptor, identity = acquire_lock(path, parent_descriptor)
    except OSError as error:
        raise SnapshotPublishError(path, SnapshotPublishFailure.LOCKED) from error
    return descriptor, _Identity(*identity)


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
    if not state.published:
        try:
            remove_owned_staging(
                state.staging,
                state.target,
                state.staging_descriptor,
                None
                if state.staging_identity is None
                else (
                    state.staging_identity.device,
                    state.staging_identity.inode,
                ),
                state.renamed,
                state.parent_descriptor,
                lambda candidate, expected: _matches(
                    candidate,
                    None if expected is None else _Identity(*expected),
                    stat.S_ISDIR,
                ),
            )
        except OSError as error:
            errors.append(error)
    try:
        _unlink_owned_file(state.lock, state.lock_identity, state.parent_descriptor)
    except OSError as error:
        errors.append(error)
    for descriptor in (
        state.staging_descriptor,
        state.lock_descriptor,
        state.parent_descriptor,
    ):
        if descriptor is not None:
            try:
                os.close(descriptor)
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
    current = _Identity(value.st_dev, value.st_ino)
    return expected_mode(value.st_mode) and current == identity


def _unlink_owned_file(
    path: Path,
    identity: _Identity,
    parent_descriptor: int,
) -> None:
    if not _matches(path, identity, stat.S_ISREG):
        raise OSError(errno.ESTALE, "lock path ownership changed")
    unlink_owned_file(
        path,
        (identity.device, identity.inode),
        parent_descriptor,
    )
