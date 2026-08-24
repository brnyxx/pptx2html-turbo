from __future__ import annotations

import errno
import os
import stat
import uuid
from collections.abc import Callable
from pathlib import Path

from evaluate import multiformat_snapshot_filesystem as snapshot_filesystem


def remove_owned_empty_directory(
    path: Path,
    identity: tuple[int, int],
    parent_descriptor: int,
) -> None:
    information = os.stat(path.name, dir_fd=parent_descriptor, follow_symlinks=False)
    if (
        not stat.S_ISDIR(information.st_mode)
        or snapshot_filesystem._identity(information) != identity
    ):
        raise OSError(errno.ESTALE, "staging path ownership changed")
    tombstone = f".cleanup-{uuid.uuid4().hex}"
    os.rename(
        path.name,
        tombstone,
        src_dir_fd=parent_descriptor,
        dst_dir_fd=parent_descriptor,
    )
    try:
        descriptor = os.open(
            tombstone,
            snapshot_filesystem._DIRECTORY_FLAGS | snapshot_filesystem._NOFOLLOW_FLAGS,
            dir_fd=parent_descriptor,
        )
        try:
            if snapshot_filesystem._identity(os.fstat(descriptor)) != identity:
                raise OSError(errno.ESTALE, "staging path ownership changed")
        finally:
            os.close(descriptor)
        os.rmdir(tombstone, dir_fd=parent_descriptor)
    except OSError as error:
        try:
            os.stat(path.name, dir_fd=parent_descriptor, follow_symlinks=False)
        except FileNotFoundError:
            try:
                snapshot_filesystem.atomic_rename_noreplace(
                    Path(tombstone),
                    Path(path.name),
                    parent_descriptor,
                )
            except OSError as restore_error:
                raise restore_error from error
        raise


def remove_owned_staging(
    staging: Path | None,
    target: Path,
    descriptor: int | None,
    identity: tuple[int, int] | None,
    renamed: bool,
    parent_descriptor: int,
    matches: Callable[[Path | None, tuple[int, int] | None], bool],
) -> None:
    if descriptor is None:
        if staging is not None and identity is not None:
            remove_owned_empty_directory(staging, identity, parent_descriptor)
        return
    path = target if renamed else staging
    if path is None or identity is None:
        raise OSError(errno.ESTALE, "staging path ownership changed")
    snapshot_filesystem.remove_owned_directory(
        path,
        identity,
        descriptor,
        parent_descriptor,
        matches,
    )
