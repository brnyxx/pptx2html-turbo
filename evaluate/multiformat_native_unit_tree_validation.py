from __future__ import annotations

import os
import stat
from pathlib import Path

from evaluate.multiformat_native_unit_stable_validation import (
    StableFile,
    stable_file_at,
)
from evaluate.multiformat_native_unit_types import NativeUnitError, NativeUnitFailure


def validate_inventory_tree(
    root_descriptor: int,
    expected_files: tuple[tuple[str, StableFile], ...],
) -> None:
    expected = dict(expected_files)
    if len(expected) != len(expected_files):
        raise _failure("inventory file binding is duplicated")
    actual: set[str] = set()
    identities: set[tuple[int, int]] = set()
    for directory, directories, files, descriptor in os.fwalk(
        ".",
        topdown=True,
        follow_symlinks=False,
        dir_fd=root_descriptor,
    ):
        directories.sort()
        files.sort()
        for name in directories:
            value = os.stat(name, dir_fd=descriptor, follow_symlinks=False)
            if not stat.S_ISDIR(value.st_mode):
                raise _failure("inventory tree contains an invalid directory")
        for name in files:
            value = os.stat(name, dir_fd=descriptor, follow_symlinks=False)
            if not stat.S_ISREG(value.st_mode) or value.st_nlink != 1:
                raise _failure("inventory tree contains an invalid file")
            identity = (value.st_dev, value.st_ino)
            if identity in identities:
                raise _failure("inventory tree reuses a file inode")
            identities.add(identity)
            relative = Path(directory, name).as_posix().removeprefix("./")
            actual.add(relative)
    if actual != set(expected):
        raise _failure("inventory file set differs")
    for relative, expected_state in expected_files:
        current = stable_file_at(
            root_descriptor,
            relative,
            executable=False,
            maximum=expected_state[2],
        )
        if current != expected_state:
            raise _failure("inventory file identity changed")


def _failure(detail: str) -> NativeUnitError:
    return NativeUnitError(NativeUnitFailure.OUTPUT_INVALID, None, None, detail)
