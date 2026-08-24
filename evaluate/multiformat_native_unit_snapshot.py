from __future__ import annotations

import os
import secrets
import stat
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path

from evaluate.multiformat_candidate_process import (
    CandidateProcessError,
    CandidateProcessFailure,
)
from evaluate.multiformat_native_unit_io import no_follow, read_descriptor
from evaluate.multiformat_native_unit_trusted import NativeTrustedExecutable

_EXECUTION_DIRECTORY_MODE = stat.S_IRUSR | stat.S_IXUSR


@dataclass(frozen=True, slots=True)
class NativeExecutableSnapshot:
    path: Path
    root_name: str
    root_descriptor: int
    binary_directory_descriptor: int
    binary_descriptor: int
    parent_descriptor: int
    root_identity: tuple[int, int]
    library_linked: bool


def materialize_binary(
    trusted: NativeTrustedExecutable, cwd: Path
) -> NativeExecutableSnapshot:
    content = trusted.content
    parent_descriptor = -1
    root_descriptor = -1
    binary_directory = -1
    binary_descriptor = -1
    root: Path | None = None
    try:
        _verify_trusted_source(trusted)
        parent_descriptor = os.open(
            cwd, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0) | no_follow()
        )
        root = Path(tempfile.mkdtemp(prefix=".native-execution-", dir=cwd))
        root_name = root.name
        root_descriptor = os.open(
            root_name,
            os.O_RDONLY | getattr(os, "O_DIRECTORY", 0) | no_follow(),
            dir_fd=parent_descriptor,
        )
        root_identity = _identity(os.fstat(root_descriptor))
        os.mkdir("bin", stat.S_IRWXU, dir_fd=root_descriptor)
        binary_directory = os.open(
            "bin",
            os.O_RDONLY | getattr(os, "O_DIRECTORY", 0) | no_follow(),
            dir_fd=root_descriptor,
        )
        library = trusted.path.parent.parent / "lib"
        library_linked = library.is_dir()
        if library_linked:
            os.symlink(library.as_posix(), "lib", dir_fd=root_descriptor)
        snapshot_name = f".trusted-executable-{_token()}"
        binary_descriptor = os.open(
            snapshot_name,
            os.O_RDWR | os.O_CREAT | os.O_EXCL | no_follow(),
            stat.S_IRWXU,
            dir_fd=binary_directory,
        )
        _write_all(binary_descriptor, content)
        _verify_snapshot(binary_descriptor, content)
        if _requires_platform_signing(trusted.path):
            _ad_hoc_sign(root / "bin" / snapshot_name)
        os.fchmod(binary_directory, _EXECUTION_DIRECTORY_MODE)
        os.fchmod(root_descriptor, _EXECUTION_DIRECTORY_MODE)
        return NativeExecutableSnapshot(
            root / "bin" / snapshot_name,
            root_name,
            root_descriptor,
            binary_directory,
            binary_descriptor,
            parent_descriptor,
            root_identity,
            library_linked,
        )
    except OSError as error:
        _close(binary_descriptor)
        _close(binary_directory)
        _close(root_descriptor)
        _close(parent_descriptor)
        if root is not None:
            _remove_partial(root)
        raise CandidateProcessError(
            CandidateProcessFailure.EXECUTABLE_UNTRUSTED
        ) from error


def release_binary(snapshot: NativeExecutableSnapshot) -> None:
    active = sys.exception()
    try:
        os.fchmod(snapshot.root_descriptor, stat.S_IRWXU)
        os.fchmod(snapshot.binary_directory_descriptor, stat.S_IRWXU)
        os.unlink(snapshot.path.name, dir_fd=snapshot.binary_directory_descriptor)
        if snapshot.library_linked:
            os.unlink("lib", dir_fd=snapshot.root_descriptor)
        os.rmdir("bin", dir_fd=snapshot.root_descriptor)
        if not _entry_matches(
            snapshot.parent_descriptor, snapshot.root_name, snapshot.root_identity
        ):
            raise OSError("execution directory changed")
        os.rmdir(snapshot.root_name, dir_fd=snapshot.parent_descriptor)
    except OSError as error:
        failure = CandidateProcessError(CandidateProcessFailure.EXECUTABLE_UNTRUSTED)
        if active is not None:
            active.add_note(str(failure))
            active.add_note(str(error))
        else:
            raise failure from error
    finally:
        _close(snapshot.binary_descriptor)
        _close(snapshot.binary_directory_descriptor)
        _close(snapshot.root_descriptor)
        _close(snapshot.parent_descriptor)


def _verify_trusted_source(trusted: NativeTrustedExecutable) -> None:
    value = os.fstat(trusted.descriptor)
    _ = os.lseek(trusted.descriptor, 0, os.SEEK_SET)
    content = read_descriptor(trusted.descriptor)
    _ = os.lseek(trusted.descriptor, 0, os.SEEK_SET)
    if _identity(value) != trusted.identity or content != trusted.content:
        raise OSError("trusted executable changed before materialization")


def _verify_snapshot(descriptor: int, content: bytes) -> None:
    _ = os.lseek(descriptor, 0, os.SEEK_SET)
    if read_descriptor(descriptor) != content:
        raise OSError("trusted executable snapshot changed")
    value = os.fstat(descriptor)
    if value.st_size != len(content) or not stat.S_ISREG(value.st_mode):
        raise OSError("trusted executable snapshot is invalid")
    _ = os.lseek(descriptor, 0, os.SEEK_SET)


def _requires_platform_signing(source: Path) -> bool:
    return sys.platform == "darwin" and source.as_posix().startswith(
        ("/bin/", "/usr/bin/")
    )


def _ad_hoc_sign(path: Path) -> None:
    try:
        _ = subprocess.run(
            ("/usr/bin/codesign", "--force", "--sign", "-", path.as_posix()),
            cwd=path.parent,
            env={"PATH": os.defpath, "LC_ALL": "C"},
            stdin=subprocess.DEVNULL,
            capture_output=True,
            start_new_session=True,
            timeout=5.0,
            check=True,
        )
    except (OSError, subprocess.TimeoutExpired, subprocess.CalledProcessError) as error:
        raise OSError("platform binary signing failed") from error


def _remove_partial(root: Path) -> None:
    try:
        os.chmod(root, stat.S_IRWXU)
        binary_directory = root / "bin"
        os.chmod(binary_directory, stat.S_IRWXU)
        for child in binary_directory.iterdir():
            child.unlink()
        binary_directory.rmdir()
        library = root / "lib"
        if library.is_symlink():
            library.unlink()
        root.rmdir()
    except OSError:
        return


def _entry_matches(
    directory_descriptor: int, name: str, expected: tuple[int, int]
) -> bool:
    try:
        value = os.lstat(name, dir_fd=directory_descriptor)
    except FileNotFoundError:
        return False
    return _identity(value) == expected


def _token() -> str:
    return secrets.token_hex(16)


def _close(descriptor: int) -> None:
    if descriptor >= 0:
        try:
            os.close(descriptor)
        except OSError:
            return


def _write_all(descriptor: int, content: bytes) -> None:
    offset = 0
    while offset < len(content):
        written = os.write(descriptor, content[offset:])
        if written <= 0:
            raise OSError("trusted executable write made no progress")
        offset += written


def _identity(value: os.stat_result) -> tuple[int, int]:
    return value.st_dev, value.st_ino
