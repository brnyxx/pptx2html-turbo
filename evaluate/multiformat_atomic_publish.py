from __future__ import annotations

import errno
import os
import shutil
import stat
import tempfile
from collections.abc import Callable
from dataclasses import dataclass
from enum import StrEnum
from pathlib import Path
from typing import Final

READY_NAME: Final = "READY"
READY_BYTES: Final = b"READY\n"
_UNSUPPORTED_SYNC_ERRORS: Final = frozenset(
    {
        errno.EBADF,
        errno.EINVAL,
        errno.EROFS,
        errno.ENOTSUP,
    }
)

PublishWriter = Callable[[Path], None]


class AtomicPublishFailure(StrEnum):
    DESTINATION_EXISTS = "destination-exists"
    READY_RESERVED = "ready-reserved"


@dataclass(frozen=True, slots=True)
class AtomicPublishError(Exception):
    destination: Path
    failure: AtomicPublishFailure

    def __str__(self) -> str:
        return f"atomic publication failed: {self.failure.value}: {self.destination}"


@dataclass(frozen=True, slots=True)
class _DirectoryIdentity:
    device: int
    inode: int


def atomic_publish(destination: Path, writer: PublishWriter) -> None:
    """Publish a complete directory tree and create READY only after its rename."""
    if os.path.lexists(destination):
        raise AtomicPublishError(
            destination=destination,
            failure=AtomicPublishFailure.DESTINATION_EXISTS,
        )

    destination.parent.mkdir(parents=True, exist_ok=True)
    staging = Path(
        tempfile.mkdtemp(
            prefix=f".{destination.name}.stage-",
            dir=destination.parent,
        )
    )
    staging_stat = staging.stat()
    identity = _DirectoryIdentity(
        device=staging_stat.st_dev,
        inode=staging_stat.st_ino,
    )
    complete = False
    try:
        writer(staging)
        if os.path.lexists(staging / READY_NAME):
            raise AtomicPublishError(
                destination=destination,
                failure=AtomicPublishFailure.READY_RESERVED,
            )
        _fsync_tree(staging)
        if os.path.lexists(destination):
            raise AtomicPublishError(
                destination=destination,
                failure=AtomicPublishFailure.DESTINATION_EXISTS,
            )
        os.rename(staging, destination)
        _fsync_directory(destination.parent)
        _write_ready(destination)
        _fsync_directory(destination)
        complete = True
    finally:
        if not complete:
            _remove_owned_directory(staging, identity)
            _remove_owned_directory(destination, identity)


def _fsync_tree(root: Path) -> None:
    directories: list[Path] = []
    for current, _, filenames in os.walk(root):
        directory = Path(current)
        directories.append(directory)
        for filename in filenames:
            path = directory / filename
            if path.is_file():
                _fsync_file(path)
    for directory in reversed(directories):
        _fsync_directory(directory)


def _fsync_file(path: Path) -> None:
    descriptor = os.open(path, os.O_RDONLY)
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)


def _fsync_directory(path: Path) -> None:
    if os.name == "nt":
        return
    descriptor = os.open(path, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
    try:
        try:
            os.fsync(descriptor)
        except OSError as error:
            if error.errno not in _UNSUPPORTED_SYNC_ERRORS:
                raise
    finally:
        os.close(descriptor)


def _write_ready(destination: Path) -> None:
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=f".{READY_NAME}.",
        dir=destination,
    )
    temporary = Path(temporary_name)
    try:
        with os.fdopen(descriptor, "wb") as stream:
            stream.write(READY_BYTES)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, destination / READY_NAME)
    finally:
        temporary.unlink(missing_ok=True)


def _remove_owned_directory(path: Path, identity: _DirectoryIdentity) -> None:
    try:
        path_stat = path.lstat()
    except FileNotFoundError:
        return
    if (
        stat.S_ISDIR(path_stat.st_mode)
        and path_stat.st_dev == identity.device
        and path_stat.st_ino == identity.inode
    ):
        shutil.rmtree(path)
