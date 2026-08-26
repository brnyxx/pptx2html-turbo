from __future__ import annotations

import hashlib
import os
import stat

from evaluate.multiformat_candidate_process import (
    CandidateProcessError,
    CandidateProcessFailure,
)
from evaluate.multiformat_native_unit_io import no_follow, read_descriptor, same_file
from evaluate.multiformat_native_unit_types import (
    NativeExecutableBinding,
    NativeStableFile,
)


def stable_snapshot_file(descriptor: int) -> NativeStableFile:
    before = os.fstat(descriptor)
    _ = os.lseek(descriptor, 0, os.SEEK_SET)
    content = read_descriptor(descriptor)
    after = os.fstat(descriptor)
    if not same_file(before, after) or not stat.S_ISREG(after.st_mode):
        raise OSError("execution snapshot changed while reading")
    _ = os.lseek(descriptor, 0, os.SEEK_SET)
    return NativeStableFile(
        after.st_dev,
        after.st_ino,
        after.st_size,
        after.st_mtime_ns,
        after.st_ctime_ns,
        hashlib.sha256(content).hexdigest(),
    )


def verify_executable_binding(
    binding: NativeExecutableBinding, *, full_content: bool
) -> None:
    descriptor = -1
    try:
        before = binding.path.lstat()
        if (
            stat.S_ISLNK(before.st_mode)
            or not stat.S_ISREG(before.st_mode)
            or not _matches_stable_file(before, binding.identity)
        ):
            raise OSError("execution snapshot path changed")
        if full_content:
            descriptor = os.open(binding.path, os.O_RDONLY | no_follow())
            if stable_snapshot_file(descriptor) != binding.identity:
                raise OSError("execution snapshot bytes changed")
        after = binding.path.lstat()
        if not same_file(before, after):
            raise OSError("execution snapshot changed during verification")
    except OSError as error:
        raise CandidateProcessError(
            CandidateProcessFailure.EXECUTABLE_UNTRUSTED
        ) from error
    finally:
        if descriptor >= 0:
            try:
                os.close(descriptor)
            except OSError:
                return


def _matches_stable_file(value: os.stat_result, expected: NativeStableFile) -> bool:
    return (
        value.st_dev == expected.device
        and value.st_ino == expected.inode
        and value.st_size == expected.size
        and value.st_mtime_ns == expected.modified_ns
        and value.st_ctime_ns == expected.changed_ns
    )
