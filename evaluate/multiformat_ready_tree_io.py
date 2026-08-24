"""Descriptor ownership for READY tree identity filesystem operations."""

from __future__ import annotations

import os
import sys
from collections.abc import Generator
from contextlib import contextmanager
from pathlib import Path

from evaluate.multiformat_ready_tree_types import TreeIdentityError


@contextmanager
def fd_scope(
    path: str | Path,
    flags: int,
    parent_fd: int | None,
) -> Generator[int, None, None]:
    try:
        if parent_fd is None:
            descriptor = os.open(path, flags)
        else:
            descriptor = os.open(path, flags, dir_fd=parent_fd)
    except OSError as error:
        raise TreeIdentityError(reason=f"cannot open tree entry: {path}") from error
    try:
        yield descriptor
    finally:
        primary_error = sys.exc_info()[1]
        try:
            os.close(descriptor)
        except OSError as error:
            if primary_error is not None:
                primary_error.add_note(f"cannot close tree entry: {path}: {error}")
            else:
                raise TreeIdentityError(
                    reason=f"cannot close tree entry: {path}"
                ) from error
