from __future__ import annotations

import os
from pathlib import Path

from evaluate.multiformat_snapshot_filesystem import unlink_owned_file

_NOFOLLOW_FLAGS = getattr(os, "O_NOFOLLOW", 0)


def acquire_lock(
    path: Path,
    parent_descriptor: int,
) -> tuple[int, tuple[int, int]]:
    flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL | _NOFOLLOW_FLAGS
    flags |= getattr(os, "O_CLOEXEC", 0)
    descriptor = os.open(path.name, flags, 0o600, dir_fd=parent_descriptor)
    try:
        identity = _identity(os.fstat(descriptor))
    except OSError as error:
        _recover_created_lock(path, parent_descriptor, descriptor, error)
        raise
    return descriptor, identity


def _recover_created_lock(
    path: Path,
    parent_descriptor: int,
    descriptor: int,
    error: OSError,
) -> None:
    try:
        binding = os.dup(descriptor)
        try:
            identity = _identity(os.fstat(binding))
        finally:
            os.close(binding)
        try:
            unlink_owned_file(path, identity, parent_descriptor)
        except OSError as cleanup_error:
            error.add_note(f"snapshot cleanup failed: {cleanup_error}")
    except OSError as recovery_error:
        error.add_note(f"snapshot cleanup failed: {recovery_error}")
    try:
        os.close(descriptor)
    except OSError as cleanup_error:
        error.add_note(f"snapshot cleanup failed: {cleanup_error}")


def _identity(value: os.stat_result) -> tuple[int, int]:
    return value.st_dev, value.st_ino
