from __future__ import annotations

import hashlib
import os
from pathlib import Path

from evaluate.multiformat_native_unit_types import (
    NativeStableFile,
    NativeUnitError,
    NativeUnitFailure,
    NativeUnitRequest,
)


def write_new(
    destination: Path, content: bytes, request: NativeUnitRequest
) -> NativeStableFile:
    if os.path.lexists(destination):
        raise _fail(request, "destination already exists")
    flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL | no_follow()
    try:
        descriptor = os.open(destination, flags, 0o600)
        try:
            _write_descriptor(descriptor, content)
            value = os.fstat(descriptor)
        finally:
            os.close(descriptor)
        path_value = destination.lstat()
        if not same_file(value, path_value) or value.st_size != len(content):
            raise _fail(request, "destination changed")
        return NativeStableFile(
            value.st_dev,
            value.st_ino,
            value.st_size,
            value.st_mtime_ns,
            hashlib.sha256(content).hexdigest(),
        )
    except NativeUnitError:
        raise
    except OSError as error:
        raise _fail(request, "file copy failed") from error


def read_descriptor(descriptor: int) -> bytes:
    chunks: list[bytes] = []
    while chunk := os.read(descriptor, 1024 * 1024):
        chunks.append(chunk)
    return b"".join(chunks)


def same_file(left: os.stat_result, right: os.stat_result) -> bool:
    return (
        _identity(left) == _identity(right)
        and left.st_size == right.st_size
        and left.st_mtime_ns == right.st_mtime_ns
    )


def no_follow() -> int:
    return getattr(os, "O_NOFOLLOW", 0)


def _write_descriptor(descriptor: int, content: bytes) -> None:
    offset = 0
    while offset < len(content):
        offset += os.write(descriptor, content[offset:])


def _identity(value: os.stat_result) -> tuple[int, int]:
    return value.st_dev, value.st_ino


def _fail(request: NativeUnitRequest, detail: str) -> NativeUnitError:
    return NativeUnitError(
        NativeUnitFailure.OUTPUT_INVALID,
        request.source.document_format,
        request.source.source_id,
        detail,
    )
