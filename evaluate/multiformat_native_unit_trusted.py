from __future__ import annotations

import hashlib
import os
import sys
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
    identity: tuple[int, int]


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
            path,
            descriptor,
            content,
            _is_shell_script(content),
            _identity(after),
        )
    except OSError as error:
        _close_descriptor(descriptor)
        raise CandidateProcessError(
            CandidateProcessFailure.EXECUTABLE_UNTRUSTED
        ) from error


def close_trusted_executable(descriptor: int) -> None:
    active = sys.exception()
    try:
        os.close(descriptor)
    except OSError as error:
        failure = CandidateProcessError(CandidateProcessFailure.EXECUTABLE_UNTRUSTED)
        if active is not None:
            active.add_note(str(failure))
            active.add_note(str(error))
        else:
            raise failure from error


def _close_descriptor(descriptor: int) -> None:
    if descriptor >= 0:
        try:
            os.close(descriptor)
        except OSError:
            return


def _identity(value: os.stat_result) -> tuple[int, int]:
    return value.st_dev, value.st_ino


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
