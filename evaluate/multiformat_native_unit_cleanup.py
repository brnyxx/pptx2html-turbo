from __future__ import annotations

import os
import stat
import sys
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
    try:
        workspace_descriptor = os.open(
            path.name,
            os.O_RDONLY | getattr(os, "O_DIRECTORY", 0) | no_follow(),
            dir_fd=parent_descriptor,
        )
        if identity(os.fstat(workspace_descriptor)) != expected:
            raise OSError("workspace identity changed")
        _clean_directory(workspace_descriptor)
        _before_final_rmdir(path)
        if not _entry_matches(parent_descriptor, path.name, expected):
            raise OSError("workspace replacement survived cleanup")
        os.rmdir(path.name, dir_fd=parent_descriptor)
    finally:
        if workspace_descriptor >= 0:
            os.close(workspace_descriptor)
        os.close(parent_descriptor)


def _clean_directory(directory_descriptor: int) -> None:
    with os.scandir(directory_descriptor) as entries:
        children = tuple(entries)
    for entry in children:
        child_stat = entry.stat(follow_symlinks=False)
        _before_inner_delete(directory_descriptor, entry.name)
        if stat.S_ISDIR(child_stat.st_mode):
            child_descriptor = os.open(
                entry.name,
                os.O_RDONLY | getattr(os, "O_DIRECTORY", 0) | no_follow(),
                dir_fd=directory_descriptor,
            )
            try:
                if not _same_stat(child_stat, os.fstat(child_descriptor)):
                    raise OSError("workspace child changed")
                _clean_directory(child_descriptor)
            finally:
                os.close(child_descriptor)
            if not _entry_matches(
                directory_descriptor, entry.name, identity(child_stat)
            ):
                raise OSError("workspace child changed")
            os.rmdir(entry.name, dir_fd=directory_descriptor)
        else:
            if not _entry_matches(
                directory_descriptor, entry.name, identity(child_stat)
            ):
                raise OSError("workspace child changed")
            os.unlink(entry.name, dir_fd=directory_descriptor)


def _entry_matches(
    directory_descriptor: int, name: str, expected: tuple[int, int]
) -> bool:
    try:
        value = os.lstat(name, dir_fd=directory_descriptor)
    except FileNotFoundError:
        return False
    return identity(value) == expected


def _same_stat(left: os.stat_result, right: os.stat_result) -> bool:
    return (
        identity(left) == identity(right)
        and left.st_size == right.st_size
        and left.st_mtime_ns == right.st_mtime_ns
        and left.st_ctime_ns == right.st_ctime_ns
    )


def _before_inner_delete(_directory_descriptor: int, _name: str) -> None:
    return


def _before_final_rmdir(_path: Path) -> None:
    return


def _fail(request: NativeUnitRequest, detail: str) -> NativeUnitError:
    return NativeUnitError(
        NativeUnitFailure.OUTPUT_INVALID,
        request.source.document_format,
        request.source.source_id,
        detail,
    )
