from __future__ import annotations

import hashlib
import os
import stat
from pathlib import Path

from evaluate.multiformat_native_unit_types import NativeUnitError, NativeUnitFailure

StableFile = tuple[int, int, int, int, int, str]


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


def _failure(detail: str) -> NativeUnitError:
    return NativeUnitError(NativeUnitFailure.OUTPUT_INVALID, None, None, detail)
