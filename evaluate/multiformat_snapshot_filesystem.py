from __future__ import annotations

import ctypes
import errno
import os
import stat
import sys
import uuid
from collections.abc import Callable
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


def valid_lock_namespace(value: str) -> bool:
    if not value or value[0] == "-" or value[-1] == "-":
        return False
    previous_was_separator = False
    for character in value:
        is_letter = "a" <= character <= "z"
        is_digit = "0" <= character <= "9"
        if character == "-":
            if previous_was_separator:
                return False
            previous_was_separator = True
        elif is_letter or is_digit:
            previous_was_separator = False
        else:
            return False
    return True


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
        try:
            os.close(descriptor)
        except OSError as cleanup_error:
            error.add_note(f"snapshot cleanup failed: {cleanup_error}")
        raise
    return descriptor, identity


def verify_directory_identity(
    path: Path,
    expected_identity: tuple[int, int],
) -> None:
    descriptor = os.open(path, _DIRECTORY_FLAGS | _NOFOLLOW_FLAGS)
    try:
        if _identity(os.fstat(descriptor)) != expected_identity:
            raise OSError(errno.ESTALE, "published directory identity changed")
    finally:
        os.close(descriptor)


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
    except OSError as error:
        try:
            os.stat(name, dir_fd=parent_descriptor, follow_symlinks=False)
        except FileNotFoundError:
            try:
                atomic_rename_noreplace(
                    Path(tombstone),
                    Path(name),
                    parent_descriptor,
                )
            except OSError as restore_error:
                raise restore_error from error
        raise


def remove_owned_directory(
    path: Path | None,
    identity: tuple[int, int] | None,
    descriptor: int,
    parent_descriptor: int,
    matches: Callable[[Path | None, tuple[int, int] | None], bool],
) -> None:
    if not matches(path, identity):
        _clean_directory_fd(descriptor)
        raise OSError(errno.ESTALE, "staging path ownership changed")
    _clean_directory_fd(descriptor)
    if not matches(path, identity):
        raise OSError(errno.ESTALE, "staging path ownership changed")
    if path is None or identity is None:
        raise OSError(errno.ESTALE, "staging path ownership changed")
    _remove_owned_entry(
        parent_descriptor,
        path.name,
        identity,
        directory=True,
    )


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
