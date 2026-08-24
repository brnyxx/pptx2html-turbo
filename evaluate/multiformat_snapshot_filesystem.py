from __future__ import annotations

import ctypes
import errno
import os
import stat
import sys
import uuid
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
        os.fsencode(staging.name),
        parent_descriptor,
        os.fsencode(target.name),
        flags,
    )
    if result == 0:
        return
    error = OSError(ctypes.get_errno(), "atomic no-replace rename failed")
    if error.errno == errno.EEXIST:
        raise FileExistsError(error.errno, error.strerror) from error
    raise error


def _clean_directory_fd(descriptor: int) -> None:
    with os.scandir(descriptor) as entries:
        children = tuple(entries)
    for entry in children:
        information = entry.stat(follow_symlinks=False)
        identity = _identity(information)
        if stat.S_ISDIR(information.st_mode) and not stat.S_ISLNK(information.st_mode):
            child = os.open(
                entry.name,
                _DIRECTORY_FLAGS | _NOFOLLOW_FLAGS,
                dir_fd=descriptor,
            )
            try:
                if _identity(os.fstat(child)) != identity:
                    raise OSError(errno.ESTALE, "staging child ownership changed")
                _clean_directory_fd(child)
            finally:
                os.close(child)
            _remove_owned_entry(descriptor, entry.name, identity, directory=True)
        else:
            child = os.open(
                entry.name,
                os.O_RDONLY | _NOFOLLOW_FLAGS,
                dir_fd=descriptor,
            )
            try:
                if _identity(os.fstat(child)) != identity:
                    raise OSError(errno.ESTALE, "staging child ownership changed")
            finally:
                os.close(child)
            _remove_owned_entry(descriptor, entry.name, identity, directory=False)


def _remove_owned_entry(
    parent_descriptor: int,
    name: str,
    identity: tuple[int, int],
    *,
    directory: bool,
) -> None:
    tombstone = f".cleanup-{uuid.uuid4().hex}"
    os.rename(
        name,
        tombstone,
        src_dir_fd=parent_descriptor,
        dst_dir_fd=parent_descriptor,
    )
    try:
        flags = (_DIRECTORY_FLAGS if directory else os.O_RDONLY) | _NOFOLLOW_FLAGS
        descriptor = os.open(tombstone, flags, dir_fd=parent_descriptor)
        try:
            if _identity(os.fstat(descriptor)) != identity:
                raise OSError(errno.ESTALE, "cleanup entry ownership changed")
        finally:
            os.close(descriptor)
        if directory:
            os.rmdir(tombstone, dir_fd=parent_descriptor)
        else:
            os.unlink(tombstone, dir_fd=parent_descriptor)
    except OSError:
        try:
            os.stat(name, dir_fd=parent_descriptor, follow_symlinks=False)
        except FileNotFoundError:
            os.rename(
                tombstone,
                name,
                src_dir_fd=parent_descriptor,
                dst_dir_fd=parent_descriptor,
            )
        raise


def unlink_owned_file(
    path: Path,
    identity: tuple[int, int],
    parent_descriptor: int,
) -> None:
    information = path.lstat()
    if not stat.S_ISREG(information.st_mode) or _identity(information) != identity:
        raise OSError(errno.ESTALE, "lock path ownership changed")
    descriptor = os.open(
        path.name,
        os.O_WRONLY | _NOFOLLOW_FLAGS,
        dir_fd=parent_descriptor,
    )
    try:
        if _identity(os.fstat(descriptor)) != identity:
            raise OSError(errno.ESTALE, "lock inode changed")
    finally:
        os.close(descriptor)
    _remove_owned_entry(parent_descriptor, path.name, identity, directory=False)


def _identity(value: os.stat_result) -> tuple[int, int]:
    return value.st_dev, value.st_ino
