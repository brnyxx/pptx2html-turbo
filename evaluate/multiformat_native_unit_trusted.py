from __future__ import annotations

import hashlib
import os
import stat
import tempfile
from dataclasses import dataclass
from pathlib import Path

from evaluate.multiformat_candidate_process import (
    CandidateProcessError,
    CandidateProcessFailure,
)
from evaluate.multiformat_native_unit_io import no_follow, read_descriptor, same_file
from evaluate.multiformat_native_unit_types import NativeStableFile


@dataclass(frozen=True, slots=True)
class NativeTrustedExecutable:
    path: Path
    descriptor: int
    content: bytes
    shell_script: bool


def open_trusted_executable(
    path: Path, expected: NativeStableFile
) -> NativeTrustedExecutable:
    descriptor = -1
    try:
        descriptor = os.open(path, os.O_RDONLY | no_follow())
        opened = os.fstat(descriptor)
        content = read_descriptor(descriptor)
        after = os.fstat(descriptor)
        if (
            not same_file(opened, after)
            or not _matches_expected(after, expected)
            or hashlib.sha256(content).hexdigest() != expected.sha256
        ):
            raise OSError("executable changed during trust binding")
        _ = os.lseek(descriptor, 0, os.SEEK_SET)
        return NativeTrustedExecutable(
            path, descriptor, content, _is_shell_script(content)
        )
    except OSError as error:
        if descriptor >= 0:
            os.close(descriptor)
        raise CandidateProcessError(
            CandidateProcessFailure.EXECUTABLE_UNTRUSTED
        ) from error


def materialize_binary(content: bytes, directory: Path) -> Path:
    descriptor, raw_path = tempfile.mkstemp(
        prefix=".trusted-executable-", dir=directory
    )
    path = Path(raw_path)
    try:
        _write_all(descriptor, content)
        os.fchmod(descriptor, stat.S_IRWXU)
    except OSError:
        os.close(descriptor)
        path.unlink(missing_ok=True)
        raise
    os.close(descriptor)
    return path


def _write_all(descriptor: int, content: bytes) -> None:
    offset = 0
    while offset < len(content):
        written = os.write(descriptor, content[offset:])
        if written <= 0:
            raise OSError("trusted executable write made no progress")
        offset += written


def _matches_expected(value: os.stat_result, expected: NativeStableFile) -> bool:
    return (
        value.st_dev == expected.device
        and value.st_ino == expected.inode
        and value.st_size == expected.size
        and value.st_mtime_ns == expected.modified_ns
        and value.st_ctime_ns == expected.changed_ns
    )


def _is_shell_script(content: bytes) -> bool:
    first_line = content.splitlines()[0] if content.startswith(b"#!") else b""
    return b"/sh" in first_line
