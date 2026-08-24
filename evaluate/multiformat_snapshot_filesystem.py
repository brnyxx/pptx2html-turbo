from __future__ import annotations

import ctypes
import errno
import os
import stat
import sys
from pathlib import Path

_RENAME_EXCL = 0x00000004  # macOS SDK sys/stdio.h
_RENAME_NOREPLACE = 1  # Linux renameat2(2) ABI
_ATOMIC_RENAME_SPECS = {
    "darwin": ("renameatx_np", _RENAME_EXCL),
    "linux": ("renameat2", _RENAME_NOREPLACE),
}
_DIRECTORY_FLAGS = os.O_RDONLY | getattr(os, "O_DIRECTORY", 0)
_NOFOLLOW_FLAGS = getattr(os, "O_NOFOLLOW", 0)


def atomic_rename_noreplace(
    staging: Path,
    target: Path,
    parent_descriptor: int,
) -> None:
    """Atomically rename without replacing an existing destination."""
    specification = _ATOMIC_RENAME_SPECS.get(sys.platform)
    if specification is None:
        raise OSError(errno.ENOTSUP, "atomic no-replace rename is unsupported")
    symbol, flags = specification
    libc = ctypes.CDLL(None, use_errno=True)
    function = getattr(libc, symbol, None)
    if function is None:
        raise OSError(errno.ENOTSUP, f"{symbol} is unavailable")
    function.argtypes = [
        ctypes.c_int,
        ctypes.c_char_p,
        ctypes.c_int,
        ctypes.c_char_p,
        ctypes.c_uint,
    ]
    function.restype = ctypes.c_int
    result = function(
        parent_descriptor,
        staging.name.encode(),
        parent_descriptor,
        target.name.encode(),
        flags,
    )
    if result == 0:
        return
    error = OSError(ctypes.get_errno(), "atomic no-replace rename failed")
    if error.errno == errno.EEXIST:
        raise FileExistsError(error.errno, error.strerror) from error
    raise error


def _clean_directory_fd(descriptor: int) -> None:
    for entry in os.scandir(descriptor):
        information = entry.stat(follow_symlinks=False)
        if stat.S_ISDIR(information.st_mode) and not stat.S_ISLNK(information.st_mode):
            child = os.open(
                entry.name,
                _DIRECTORY_FLAGS | _NOFOLLOW_FLAGS,
                dir_fd=descriptor,
            )
            try:
                if _identity(os.fstat(child)) != _identity(information):
                    raise OSError(errno.ESTALE, "staging child ownership changed")
                _clean_directory_fd(child)
            finally:
                os.close(child)
            os.rmdir(entry.name, dir_fd=descriptor)
        else:
            os.unlink(entry.name, dir_fd=descriptor)


def _identity(value: os.stat_result) -> tuple[int, int]:
    return value.st_dev, value.st_ino
