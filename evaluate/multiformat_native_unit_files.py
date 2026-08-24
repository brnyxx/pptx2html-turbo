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
from evaluate.multiformat_native_unit_io import (
    no_follow,
    read_descriptor,
    same_file,
    write_new,
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


def tool_identity(path: Path, request: NativeUnitRequest) -> NativeStableFile:
    return stable_file(path, request, NativeUnitFailure.TOOL_MISSING)


def verify_tool(
    path: Path, expected: NativeStableFile, request: NativeUnitRequest
) -> None:
    actual = tool_identity(path, request)
    if actual != expected:
        raise fail(
            request, NativeUnitFailure.OUTPUT_INVALID, "tool changed during capture"
        )


def stable_file(
    path: Path,
    request: NativeUnitRequest,
    failure: NativeUnitFailure,
    maximum: int | None = None,
) -> NativeStableFile:
    return stable_bytes(path, request, failure, maximum)[0]


def stable_bytes(
    path: Path,
    request: NativeUnitRequest,
    failure: NativeUnitFailure,
    maximum: int | None = None,
) -> tuple[NativeStableFile, bytes]:
    try:
        before = path.lstat()
        if stat.S_ISLNK(before.st_mode) or not stat.S_ISREG(before.st_mode):
            raise fail(request, failure, "file is not regular")
        if maximum is not None and before.st_size > maximum:
            raise fail(request, NativeUnitFailure.OUTPUT_OVERSIZE, "file exceeds bound")
        descriptor = os.open(path, os.O_RDONLY | no_follow())
        try:
            opened = os.fstat(descriptor)
            if not same_file(before, opened):
                raise fail(
                    request,
                    NativeUnitFailure.OUTPUT_INVALID,
                    "file changed during read",
                )
            content = read_descriptor(descriptor)
            after = os.fstat(descriptor)
        finally:
            os.close(descriptor)
        if maximum is not None and len(content) > maximum:
            raise fail(request, NativeUnitFailure.OUTPUT_OVERSIZE, "file exceeds bound")
        if not same_file(opened, after) or len(content) != before.st_size:
            raise fail(
                request, NativeUnitFailure.OUTPUT_INVALID, "file changed during capture"
            )
        digest = hashlib.sha256(content).hexdigest()
        return (
            NativeStableFile(
                before.st_dev,
                before.st_ino,
                before.st_size,
                before.st_mtime_ns,
                digest,
            ),
            content,
        )
    except FileNotFoundError as error:
        raise fail(request, failure, "file is missing") from error
    except NativeUnitError:
        raise
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
) -> NativeStableFile:
    if os.path.lexists(destination):
        raise fail(
            request, NativeUnitFailure.OUTPUT_INVALID, "destination already exists"
        )
    try:
        source_file, content = stable_bytes(
            source, request, NativeUnitFailure.OUTPUT_INVALID
        )
        if source_file != expected:
            raise fail(
                request, NativeUnitFailure.OUTPUT_INVALID, "file changed during copy"
            )
        return write_new(destination, content, request)
    except NativeUnitError:
        raise
    except (FileNotFoundError, OSError) as error:
        raise fail(
            request, NativeUnitFailure.OUTPUT_INVALID, "file copy failed"
        ) from error


def write_snapshot(
    destination: Path, content: bytes, request: NativeUnitRequest
) -> NativeStableFile:
    return write_new(destination, content, request)


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


def cleanup_workspace(
    path: Path, expected: tuple[int, int], request: NativeUnitRequest
) -> None:
    active = sys.exception()
    cleanup_error: NativeUnitError | None = None
    cleanup_cause: OSError | None = None
    try:
        value = path.lstat()
        if identity(value) != expected or not stat.S_ISDIR(value.st_mode):
            cleanup_error = fail(
                request, NativeUnitFailure.OUTPUT_INVALID, "workspace identity changed"
            )
        else:
            _ = shutil.rmtree(path)
            if os.path.lexists(path):
                cleanup_error = fail(
                    request, NativeUnitFailure.OUTPUT_INVALID, "workspace remains"
                )
    except FileNotFoundError as error:
        cleanup_error = fail(
            request, NativeUnitFailure.OUTPUT_INVALID, "workspace disappeared"
        )
        cleanup_cause = error
    except OSError as error:
        cleanup_error = fail(
            request, NativeUnitFailure.OUTPUT_INVALID, "workspace cleanup failed"
        )
        cleanup_cause = error
    if cleanup_error is not None:
        if active is not None:
            active.add_note(f"{cleanup_error}: {cleanup_cause}")
        else:
            if cleanup_cause is not None:
                raise cleanup_error from cleanup_cause
            raise cleanup_error
