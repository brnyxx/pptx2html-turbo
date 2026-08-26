"""Create-only publication of one validated evidence file.

`multiformat_snapshot_publish` owns the same discipline for directory trees. A
single JSON file needs the file-shaped version of it: the pending inode is
created with `O_CREAT|O_EXCL|O_NOFOLLOW`, kept open for the whole attempt, and
published with the platform no-replace rename from
`multiformat_snapshot_filesystem`. Every trust decision is taken on the retained
descriptor, never on the pending path, so a substituted name can neither be
followed nor published.
"""

from __future__ import annotations

import errno
import os
import stat
from collections.abc import Callable
from dataclasses import dataclass
from enum import StrEnum
from pathlib import Path

from evaluate.multiformat_snapshot_filesystem import atomic_rename_noreplace

_DIRECTORY_FLAGS = os.O_RDONLY | getattr(os, "O_DIRECTORY", 0)
_NOFOLLOW_FLAGS = getattr(os, "O_NOFOLLOW", 0)
_CLOEXEC_FLAGS = getattr(os, "O_CLOEXEC", 0)
_CREATE_FLAGS = os.O_RDWR | os.O_CREAT | os.O_EXCL | _NOFOLLOW_FLAGS | _CLOEXEC_FLAGS
_FILE_MODE = 0o644

FileValidator = Callable[[Path], None]


class MetricFilePublishFailure(StrEnum):
    DESTINATION_EXISTS = "destination-exists"
    PENDING_EXISTS = "pending-exists"
    PENDING_CHANGED = "pending-changed"
    PUBLICATION_FAILED = "publication-failed"


@dataclass(frozen=True, slots=True)
class MetricFilePublishError(Exception):
    path: Path
    failure: MetricFilePublishFailure

    def __str__(self) -> str:
        return f"file publication failed: {self.failure.value}: {self.path}"


def publish_created_file(
    destination: Path,
    payload: bytes,
    validate: FileValidator,
) -> None:
    """Publish `payload` as a newly created `destination` that never existed."""
    parent = destination.parent
    parent.mkdir(parents=True, exist_ok=True)
    parent_descriptor = os.open(
        parent.resolve(strict=True), _DIRECTORY_FLAGS | _NOFOLLOW_FLAGS
    )
    try:
        _publish(destination, payload, validate, parent_descriptor)
    finally:
        os.close(parent_descriptor)


def _publish(
    destination: Path,
    payload: bytes,
    validate: FileValidator,
    parent_descriptor: int,
) -> None:
    if os.path.lexists(destination):
        raise MetricFilePublishError(
            destination, MetricFilePublishFailure.DESTINATION_EXISTS
        )
    pending = destination.with_name(f".{destination.name}.pending")
    descriptor = _create_pending(pending, parent_descriptor)
    published = False
    try:
        _write_payload(descriptor, payload, pending)
        identity = _pinned_identity(descriptor, pending)
        _require_same_inode(pending, identity, parent_descriptor)
        validate(pending)
        if _pinned_identity(descriptor, pending) != identity:
            raise MetricFilePublishError(
                pending, MetricFilePublishFailure.PENDING_CHANGED
            )
        _require_same_inode(pending, identity, parent_descriptor)
        _publish_pending(pending, destination, parent_descriptor)
        published = True
        _require_same_inode(destination, identity, parent_descriptor)
    finally:
        if not published:
            _discard_pending(pending, _identity_of(descriptor), parent_descriptor)
        os.close(descriptor)


def _identity_of(descriptor: int) -> tuple[int, int] | None:
    try:
        value = os.fstat(descriptor)
    except OSError:
        return None
    return value.st_dev, value.st_ino


def _create_pending(pending: Path, parent_descriptor: int) -> int:
    try:
        return os.open(
            pending.name, _CREATE_FLAGS, _FILE_MODE, dir_fd=parent_descriptor
        )
    except FileExistsError as error:
        raise MetricFilePublishError(
            pending, MetricFilePublishFailure.PENDING_EXISTS
        ) from error
    except OSError as error:
        failure = (
            MetricFilePublishFailure.PENDING_EXISTS
            if error.errno in {errno.ELOOP, errno.EMLINK}
            else MetricFilePublishFailure.PUBLICATION_FAILED
        )
        raise MetricFilePublishError(pending, failure) from error


def _write_payload(descriptor: int, payload: bytes, pending: Path) -> None:
    try:
        view = memoryview(payload)
        while view:
            view = view[os.write(descriptor, view) :]
        os.fsync(descriptor)
    except OSError as error:
        raise MetricFilePublishError(
            pending, MetricFilePublishFailure.PUBLICATION_FAILED
        ) from error


def _pinned_identity(descriptor: int, pending: Path) -> tuple[int, int]:
    value = os.fstat(descriptor)
    if not stat.S_ISREG(value.st_mode) or value.st_nlink != 1:
        raise MetricFilePublishError(pending, MetricFilePublishFailure.PENDING_CHANGED)
    return value.st_dev, value.st_ino


def _require_same_inode(
    path: Path,
    identity: tuple[int, int],
    parent_descriptor: int,
) -> None:
    try:
        value = os.stat(path.name, dir_fd=parent_descriptor, follow_symlinks=False)
    except OSError as error:
        raise MetricFilePublishError(
            path, MetricFilePublishFailure.PENDING_CHANGED
        ) from error
    if (
        not stat.S_ISREG(value.st_mode)
        or value.st_nlink != 1
        or (value.st_dev, value.st_ino) != identity
    ):
        raise MetricFilePublishError(path, MetricFilePublishFailure.PENDING_CHANGED)


def _publish_pending(
    pending: Path,
    destination: Path,
    parent_descriptor: int,
) -> None:
    try:
        atomic_rename_noreplace(pending, destination, parent_descriptor)
        os.fsync(parent_descriptor)
    except FileExistsError as error:
        raise MetricFilePublishError(
            destination, MetricFilePublishFailure.DESTINATION_EXISTS
        ) from error
    except OSError as error:
        raise MetricFilePublishError(
            destination, MetricFilePublishFailure.PUBLICATION_FAILED
        ) from error


def _discard_pending(
    pending: Path,
    identity: tuple[int, int] | None,
    parent_descriptor: int,
) -> None:
    if identity is None:
        return
    try:
        value = os.stat(pending.name, dir_fd=parent_descriptor, follow_symlinks=False)
    except OSError:
        return
    if (value.st_dev, value.st_ino) == identity:
        os.unlink(pending.name, dir_fd=parent_descriptor)
