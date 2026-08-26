from __future__ import annotations

import hashlib
import os
import stat
from pathlib import Path
from types import TracebackType
from typing import Literal, final

from evaluate.multiformat_native_unit_types import NativeUnitError, NativeUnitFailure

StableFile = tuple[int, int, int, int, int, str]
_DIRECTORY_FLAGS = os.O_RDONLY | getattr(os, "O_DIRECTORY", 0)
_NOFOLLOW_FLAGS = getattr(os, "O_NOFOLLOW", 0)


@final
class _StableDirectory:
    def __init__(self, path: Path) -> None:
        self._path = path
        self._descriptor: int | None = None

    def __enter__(self) -> int:
        before = self._path.lstat()
        if not stat.S_ISDIR(before.st_mode):
            raise _failure("inventory root is not a directory")
        descriptor = os.open(
            self._path,
            _DIRECTORY_FLAGS | _NOFOLLOW_FLAGS,
        )
        opened = os.fstat(descriptor)
        if not _same_directory(before, opened):
            os.close(descriptor)
            raise _failure("inventory root changed while opening")
        self._descriptor = descriptor
        return descriptor

    def __exit__(
        self,
        exception_type: type[BaseException] | None,
        exception: BaseException | None,
        traceback: TracebackType | None,
    ) -> Literal[False]:
        _ = exception_type, traceback
        descriptor = self._descriptor
        if descriptor is None:
            return False
        try:
            if exception is None:
                opened = os.fstat(descriptor)
                final = self._path.lstat()
                if not _same_directory(opened, final):
                    raise _failure("inventory root changed during validation")
        finally:
            os.close(descriptor)
            self._descriptor = None
        return False


def open_stable_directory(path: Path) -> _StableDirectory:
    return _StableDirectory(path)


def stable_file(
    path: Path,
    *,
    executable: bool,
    maximum: int,
) -> StableFile:
    return stable_bytes(
        path,
        executable=executable,
        maximum=maximum,
    )[0]


def stable_file_at(
    root_descriptor: int,
    relative_path: str,
    *,
    executable: bool,
    maximum: int,
) -> StableFile:
    return stable_bytes_at(
        root_descriptor,
        relative_path,
        executable=executable,
        maximum=maximum,
    )[0]


def stable_bytes_at(
    root_descriptor: int,
    relative_path: str,
    *,
    executable: bool,
    maximum: int,
) -> tuple[StableFile, bytes]:
    components = _relative_components(relative_path)
    parent_descriptor = root_descriptor
    opened_parents: list[int] = []
    descriptor: int | None = None
    try:
        for component in components[:-1]:
            current = os.open(
                component,
                _DIRECTORY_FLAGS | _NOFOLLOW_FLAGS,
                dir_fd=parent_descriptor,
            )
            opened = os.fstat(current)
            if not stat.S_ISDIR(opened.st_mode):
                os.close(current)
                raise _failure("inventory path component is not a directory")
            opened_parents.append(current)
            parent_descriptor = current
        filename = components[-1]
        before = os.stat(
            filename,
            dir_fd=parent_descriptor,
            follow_symlinks=False,
        )
        if (
            not stat.S_ISREG(before.st_mode)
            or before.st_nlink != 1
            or not 0 < before.st_size <= maximum
        ):
            raise _failure("tool or evidence file is invalid")
        descriptor = os.open(
            filename,
            os.O_RDONLY | _NOFOLLOW_FLAGS,
            dir_fd=parent_descriptor,
        )
        opened = os.fstat(descriptor)
        if executable and not bool(opened.st_mode & 0o111):
            raise _failure("tool or evidence file is invalid")
        chunks: list[bytes] = []
        while chunk := os.read(descriptor, 1024 * 1024):
            chunks.append(chunk)
        after = os.fstat(descriptor)
        final = os.stat(
            filename,
            dir_fd=parent_descriptor,
            follow_symlinks=False,
        )
        if not (
            _same_file(before, opened)
            and _same_file(opened, after)
            and _same_file(after, final)
        ):
            raise _failure("tool or evidence file changed during validation")
        content = b"".join(chunks)
        if len(content) != before.st_size:
            raise _failure("tool or evidence file size differs")
        return (
            (
                before.st_dev,
                before.st_ino,
                before.st_size,
                before.st_mtime_ns,
                before.st_ctime_ns,
                hashlib.sha256(content).hexdigest(),
            ),
            content,
        )
    finally:
        if descriptor is not None:
            os.close(descriptor)
        for opened_parent in reversed(opened_parents):
            os.close(opened_parent)


def stable_bytes(
    path: Path,
    *,
    executable: bool,
    maximum: int,
) -> tuple[StableFile, bytes]:
    before = path.lstat()
    if (
        not stat.S_ISREG(before.st_mode)
        or before.st_nlink != 1
        or not 0 < before.st_size <= maximum
        or (executable and not os.access(path, os.X_OK))
    ):
        raise _failure("tool or evidence file is invalid")
    descriptor = os.open(path, os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0))
    try:
        opened = os.fstat(descriptor)
        chunks: list[bytes] = []
        while chunk := os.read(descriptor, 1024 * 1024):
            chunks.append(chunk)
        after = os.fstat(descriptor)
    finally:
        os.close(descriptor)
    final = path.lstat()
    if not (
        _same_file(before, opened)
        and _same_file(opened, after)
        and _same_file(after, final)
    ):
        raise _failure("tool or evidence file changed during validation")
    content = b"".join(chunks)
    if len(content) != before.st_size:
        raise _failure("tool or evidence file size differs")
    return (
        (
            before.st_dev,
            before.st_ino,
            before.st_size,
            before.st_mtime_ns,
            before.st_ctime_ns,
            hashlib.sha256(content).hexdigest(),
        ),
        content,
    )


def _same_file(first: os.stat_result, second: os.stat_result) -> bool:
    return (
        first.st_dev,
        first.st_ino,
        first.st_size,
        first.st_mtime_ns,
        first.st_ctime_ns,
        first.st_nlink,
    ) == (
        second.st_dev,
        second.st_ino,
        second.st_size,
        second.st_mtime_ns,
        second.st_ctime_ns,
        second.st_nlink,
    )


def _relative_components(relative_path: str) -> tuple[str, ...]:
    components = tuple(relative_path.split("/"))
    if (
        not components
        or relative_path.startswith("/")
        or "\\" in relative_path
        or any(component in {"", ".", ".."} for component in components)
    ):
        raise _failure("inventory evidence path is invalid")
    return components


def _same_directory(first: os.stat_result, second: os.stat_result) -> bool:
    return (
        stat.S_ISDIR(first.st_mode)
        and stat.S_ISDIR(second.st_mode)
        and (first.st_dev, first.st_ino) == (second.st_dev, second.st_ino)
    )


def _failure(detail: str) -> NativeUnitError:
    return NativeUnitError(NativeUnitFailure.OUTPUT_INVALID, None, None, detail)
