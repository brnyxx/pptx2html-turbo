from __future__ import annotations

import ctypes
import errno
import os
import secrets
import stat
import sys
from collections.abc import Callable
from pathlib import Path
from typing import Protocol

from evaluate.multiformat_native_unit_io import no_follow
from evaluate.multiformat_native_unit_types import (
    NativeUnitError,
    NativeUnitFailure,
    NativeUnitRequest,
)

_ENTRY_NAME = ".captured-entry"
_TOMBSTONE_MODE = stat.S_IRUSR | stat.S_IWUSR | stat.S_IXUSR


class _NativeCallable(Protocol):
    def __call__(self, *arguments: int | bytes) -> int: ...


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
    try:
        _before_final_rmdir(path)
        _remove_entry(
            parent_descriptor,
            path.name,
            expected,
            expect_directory=True,
        )
    finally:
        os.close(parent_descriptor)


def _clean_directory(directory_descriptor: int) -> None:
    with os.scandir(directory_descriptor) as entries:
        children = tuple(entries)
    for entry in children:
        value = entry.stat(follow_symlinks=False)
        expected = identity(value)
        _before_inner_delete(directory_descriptor, entry.name)
        _remove_entry(
            directory_descriptor,
            entry.name,
            expected,
            expect_directory=stat.S_ISDIR(value.st_mode),
        )


def _remove_entry(
    directory_descriptor: int,
    name: str,
    expected: tuple[int, int],
    *,
    expect_directory: bool,
) -> None:
    tombstone_name = f".native-tombstone-{secrets.token_hex(16)}"
    os.mkdir(tombstone_name, _TOMBSTONE_MODE, dir_fd=directory_descriptor)
    tombstone_descriptor = -1
    captured_descriptor = -1
    tombstone_identity: tuple[int, int] | None = None
    try:
        tombstone_descriptor = os.open(
            tombstone_name,
            os.O_RDONLY | getattr(os, "O_DIRECTORY", 0) | no_follow(),
            dir_fd=directory_descriptor,
        )
        tombstone_identity = identity(os.fstat(tombstone_descriptor))
        _rename_noreplace(
            directory_descriptor,
            name,
            tombstone_descriptor,
            _ENTRY_NAME,
        )
        captured_descriptor = os.open(
            _ENTRY_NAME,
            os.O_RDONLY | no_follow(),
            dir_fd=tombstone_descriptor,
        )
        captured = os.fstat(captured_descriptor)
        if (
            identity(captured) != expected
            or stat.S_ISDIR(captured.st_mode) is not expect_directory
        ):
            _restore_captured(tombstone_descriptor, directory_descriptor, name)
            raise OSError("captured entry identity changed")
        if expect_directory:
            _clean_directory(captured_descriptor)
        _before_tombstone_remove(tombstone_descriptor, _ENTRY_NAME)
        os.fchmod(tombstone_descriptor, stat.S_IRUSR | stat.S_IXUSR)
        if not _entry_matches(tombstone_descriptor, _ENTRY_NAME, expected):
            raise OSError("captured entry changed before removal")
        _remove_captured(tombstone_descriptor, _ENTRY_NAME, expect_directory)
    except OSError as error:
        if captured_descriptor < 0 and tombstone_descriptor >= 0:
            _restore_if_present(tombstone_descriptor, directory_descriptor, name)
        elif captured_descriptor >= 0 and _entry_matches(
            tombstone_descriptor, _ENTRY_NAME, expected
        ):
            try:
                _restore_captured(tombstone_descriptor, directory_descriptor, name)
            except OSError as restore_error:
                error.add_note(f"owned entry restore failed: {restore_error}")
        raise
    finally:
        if captured_descriptor >= 0:
            os.close(captured_descriptor)
        if tombstone_descriptor >= 0:
            os.close(tombstone_descriptor)
        if tombstone_identity is not None and _entry_matches(
            directory_descriptor, tombstone_name, tombstone_identity
        ):
            os.rmdir(tombstone_name, dir_fd=directory_descriptor)


def _restore_if_present(
    tombstone_descriptor: int, directory_descriptor: int, name: str
) -> None:
    try:
        _ = os.lstat(_ENTRY_NAME, dir_fd=tombstone_descriptor)
    except FileNotFoundError:
        return
    _restore_captured(tombstone_descriptor, directory_descriptor, name)


def _restore_captured(
    tombstone_descriptor: int, directory_descriptor: int, name: str
) -> None:
    _rename_noreplace(tombstone_descriptor, _ENTRY_NAME, directory_descriptor, name)


def _rename_noreplace(
    source_descriptor: int,
    source_name: str,
    destination_descriptor: int,
    destination_name: str,
) -> None:
    function_type = ctypes.CFUNCTYPE(
        ctypes.c_int,
        ctypes.c_int,
        ctypes.c_char_p,
        ctypes.c_int,
        ctypes.c_char_p,
        ctypes.c_uint,
        use_errno=True,
    )
    library = ctypes.CDLL(None, use_errno=True)
    if sys.platform == "darwin":
        function = _load_native_function(function_type, library, "renameatx_np")
        flags = 0x00000004
    elif sys.platform.startswith("linux"):
        function = _load_native_function(function_type, library, "renameat2")
        flags = 0x00000001
    else:
        raise OSError(errno.ENOTSUP, "no atomic no-replace rename")
    result = function(
        source_descriptor,
        os.fsencode(source_name),
        destination_descriptor,
        os.fsencode(destination_name),
        flags,
    )
    if result != 0:
        error_number = ctypes.get_errno()
        raise OSError(error_number, os.strerror(error_number))


def _load_native_function(
    function_type: Callable[..., _NativeCallable], library: ctypes.CDLL, name: str
) -> _NativeCallable:
    return function_type((name, library))


def _remove_captured(directory_descriptor: int, name: str, directory: bool) -> None:
    os.fchmod(directory_descriptor, _TOMBSTONE_MODE)
    function_type = ctypes.CFUNCTYPE(
        ctypes.c_int,
        ctypes.c_int,
        ctypes.c_char_p,
        ctypes.c_int,
        use_errno=True,
    )
    function = _load_native_function(
        function_type, ctypes.CDLL(None, use_errno=True), "unlinkat"
    )
    if directory:
        flags = 0x00000080 if sys.platform == "darwin" else 0x00000200
    else:
        flags = 0
    result = function(directory_descriptor, os.fsencode(name), flags)
    if result != 0:
        error_number = ctypes.get_errno()
        raise OSError(error_number, os.strerror(error_number))


def _entry_matches(
    directory_descriptor: int, name: str, expected: tuple[int, int]
) -> bool:
    try:
        value = os.lstat(name, dir_fd=directory_descriptor)
    except FileNotFoundError:
        return False
    return identity(value) == expected


def _before_inner_delete(_directory_descriptor: int, _name: str) -> None:
    return


def _before_final_rmdir(_path: Path) -> None:
    return


def _before_tombstone_remove(_directory_descriptor: int, _name: str) -> None:
    return


def _fail(request: NativeUnitRequest, detail: str) -> NativeUnitError:
    return NativeUnitError(
        NativeUnitFailure.OUTPUT_INVALID,
        request.source.document_format,
        request.source.source_id,
        detail,
    )
