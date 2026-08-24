from __future__ import annotations

import os
import shutil
import sys
import tempfile
from pathlib import Path

from evaluate.multiformat_native_unit_io import no_follow
from evaluate.multiformat_native_unit_types import (
    NativeUnitError,
    NativeUnitFailure,
    NativeUnitRequest,
)


def identity(value: os.stat_result) -> tuple[int, int]:
    return value.st_dev, value.st_ino


def cleanup_workspace(
    path: Path, expected: tuple[int, int], request: NativeUnitRequest
) -> None:
    active = sys.exception()
    cleanup_error: NativeUnitError | None = None
    cleanup_cause: OSError | None = None
    try:
        _remove_owned_workspace(path, expected)
    except FileNotFoundError as error:
        cleanup_error = _fail(request, "workspace disappeared")
        cleanup_cause = error
    except OSError as error:
        cleanup_error = _fail(request, "workspace cleanup failed")
        cleanup_cause = error
    if cleanup_error is None:
        return
    if active is not None:
        active.add_note(f"{cleanup_error}: {cleanup_cause}")
        return
    if cleanup_cause is not None:
        raise cleanup_error from cleanup_cause
    raise cleanup_error


def _remove_owned_workspace(path: Path, expected: tuple[int, int]) -> None:
    parent_descriptor = os.open(
        path.parent,
        os.O_RDONLY | getattr(os, "O_DIRECTORY", 0) | no_follow(),
    )
    workspace_descriptor = -1
    cleanup_root: Path | None = None
    cleanup_descriptor = -1
    try:
        workspace_descriptor = os.open(
            path.name,
            os.O_RDONLY | getattr(os, "O_DIRECTORY", 0) | no_follow(),
            dir_fd=parent_descriptor,
        )
        if identity(os.fstat(workspace_descriptor)) != expected:
            raise OSError("workspace identity changed")
        cleanup_root = Path(
            tempfile.mkdtemp(prefix=f".{path.name}.cleanup-", dir=path.parent)
        )
        cleanup_descriptor = os.open(
            cleanup_root,
            os.O_RDONLY | getattr(os, "O_DIRECTORY", 0) | no_follow(),
        )
        os.rename(
            path.name,
            path.name,
            src_dir_fd=parent_descriptor,
            dst_dir_fd=cleanup_descriptor,
        )
        moved = cleanup_root / path.name
        if identity(moved.lstat()) != expected:
            raise OSError("workspace identity changed")
        shutil.rmtree(moved)
        if os.path.lexists(path):
            raise OSError("workspace replacement survived cleanup")
        cleanup_root.rmdir()
        cleanup_root = None
    finally:
        if cleanup_descriptor >= 0:
            os.close(cleanup_descriptor)
        if workspace_descriptor >= 0:
            os.close(workspace_descriptor)
        os.close(parent_descriptor)


def _fail(request: NativeUnitRequest, detail: str) -> NativeUnitError:
    return NativeUnitError(
        NativeUnitFailure.OUTPUT_INVALID,
        request.source.document_format,
        request.source.source_id,
        detail,
    )
