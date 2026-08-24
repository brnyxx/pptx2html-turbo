from __future__ import annotations

import hashlib
import os
import shutil
import stat
import sys
from pathlib import Path

from evaluate.multiformat_candidate_fonts import (
    CandidateFontEnvironment,
    CandidateFontError,
    prepare_font_environment,
)
from evaluate.multiformat_native_unit_types import (
    NativeStableFile,
    NativeUnitError,
    NativeUnitFailure,
    NativeUnitRequest,
)

MAX_LOG_BYTES = 1024 * 1024
MAX_PDF_BYTES = 64 * 1024 * 1024


def fail(
    request: NativeUnitRequest, failure: NativeUnitFailure, detail: str
) -> NativeUnitError:
    return NativeUnitError(
        failure, request.source.document_format, request.source.source_id, detail
    )


def file_path(
    path: Path, request: NativeUnitRequest, failure: NativeUnitFailure
) -> Path:
    try:
        value = path.lstat()
        if stat.S_ISLNK(value.st_mode) or not stat.S_ISREG(value.st_mode):
            raise fail(request, failure, "path is not a regular file")
        return path.resolve(strict=True)
    except FileNotFoundError as error:
        raise fail(request, failure, "path is missing") from error
    except OSError as error:
        raise fail(request, failure, "path cannot be read") from error


def tool_path(path: Path, request: NativeUnitRequest) -> Path:
    resolved = file_path(path, request, NativeUnitFailure.TOOL_MISSING)
    if not os.access(resolved, os.X_OK):
        raise fail(request, NativeUnitFailure.TOOL_MISSING, "tool is not executable")
    return resolved


def stable_file(
    path: Path,
    request: NativeUnitRequest,
    failure: NativeUnitFailure,
    maximum: int | None = None,
) -> NativeStableFile:
    try:
        before = path.lstat()
        if stat.S_ISLNK(before.st_mode) or not stat.S_ISREG(before.st_mode):
            raise fail(request, failure, "file is not regular")
        if maximum is not None and before.st_size > maximum:
            raise fail(request, NativeUnitFailure.OUTPUT_OVERSIZE, "file exceeds bound")
        digest = _sha256(path)
        after = path.lstat()
        if (
            identity(before) != identity(after)
            or before.st_size != after.st_size
            or before.st_mtime_ns != after.st_mtime_ns
        ):
            raise fail(
                request, NativeUnitFailure.OUTPUT_INVALID, "file changed during capture"
            )
        return NativeStableFile(
            before.st_dev, before.st_ino, before.st_size, before.st_mtime_ns, digest
        )
    except FileNotFoundError as error:
        raise fail(request, failure, "file is missing") from error
    except OSError as error:
        raise fail(
            request, NativeUnitFailure.OUTPUT_INVALID, "file cannot be read"
        ) from error


def output_file(path: Path, request: NativeUnitRequest) -> NativeStableFile:
    value = stable_file(path, request, NativeUnitFailure.OUTPUT_MISSING, MAX_PDF_BYTES)
    if value.size == 0:
        raise fail(request, NativeUnitFailure.OUTPUT_MISSING, "PDF output is empty")
    try:
        with path.open("rb") as handle:
            header = handle.read(5)
        if header != b"%PDF-":
            raise fail(
                request, NativeUnitFailure.OUTPUT_INVALID, "PDF output is malformed"
            )
    except OSError as error:
        raise fail(
            request, NativeUnitFailure.OUTPUT_INVALID, "PDF output is unreadable"
        ) from error
    return value


def verify_file(
    expected: NativeStableFile, path: Path, request: NativeUnitRequest
) -> None:
    actual = stable_file(path, request, NativeUnitFailure.OUTPUT_INVALID)
    if actual != expected:
        raise fail(
            request, NativeUnitFailure.OUTPUT_INVALID, "file changed during capture"
        )


def copy_stable(
    source: Path,
    destination: Path,
    expected: NativeStableFile,
    request: NativeUnitRequest,
) -> None:
    try:
        if os.path.lexists(destination):
            raise fail(
                request, NativeUnitFailure.OUTPUT_INVALID, "destination already exists"
            )
        with source.open("rb") as handle:
            before = os.fstat(handle.fileno())
            content = handle.read()
            after = os.fstat(handle.fileno())
        if (
            identity(before) != identity(after)
            or len(content) != expected.size
            or hashlib.sha256(content).hexdigest() != expected.sha256
        ):
            raise fail(
                request, NativeUnitFailure.OUTPUT_INVALID, "file changed during copy"
            )
        _ = destination.write_bytes(content)
        verify_file(expected, source, request)
    except NativeUnitError:
        raise
    except (FileNotFoundError, OSError) as error:
        raise fail(
            request, NativeUnitFailure.OUTPUT_INVALID, "file copy failed"
        ) from error


def font_environment(
    request: NativeUnitRequest, workspace: Path
) -> CandidateFontEnvironment:
    try:
        return prepare_font_environment(
            request.runtime.font_bundle, workspace / "font-runtime"
        )
    except (CandidateFontError, OSError, UnicodeError, ValueError) as error:
        raise fail(
            request, NativeUnitFailure.FONT_INVALID, "font environment failed"
        ) from error


def identity(value: os.stat_result) -> tuple[int, int]:
    return value.st_dev, value.st_ino


def cleanup_workspace(path: Path, expected: tuple[int, int]) -> None:
    active = sys.exception()
    try:
        value = path.lstat()
        if identity(value) == expected and stat.S_ISDIR(value.st_mode):
            _ = shutil.rmtree(path)
    except FileNotFoundError:
        return
    except OSError as error:
        if active is not None:
            active.add_note(f"workspace cleanup failed: {error}")
        else:
            raise


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        while chunk := source.read(1024 * 1024):
            digest.update(chunk)
    return digest.hexdigest()
