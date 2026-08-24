from __future__ import annotations

import os
import platform
from pathlib import Path

from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_native_unit_files import fail as _fail
from evaluate.multiformat_native_unit_files import file_path, stable_file
from evaluate.multiformat_native_unit_observation import Captured, capture
from evaluate.multiformat_native_unit_process import run_native_process
from evaluate.multiformat_native_unit_types import (
    NativeObservation,
    NativeProcessRunner,
    NativeRouteKind,
    NativeRouteSelection,
    NativeUnitError,
    NativeUnitFailure,
    NativeUnitRequest,
)
from evaluate.multiformat_reference_routing import ToolRole
from evaluate.multiformat_schema import sha256_file
from evaluate.multiformat_snapshot_publish import SnapshotPublishError, publish_snapshot


def capture_native_observation(
    request: NativeUnitRequest, runner: NativeProcessRunner = run_native_process
) -> NativeObservation:
    _validate(request)
    source = file_path(request.source.path, request, NativeUnitFailure.SOURCE_MISSING)
    source_file = stable_file(source, request, NativeUnitFailure.SOURCE_INVALID)
    route = _route(request)
    try:
        request.observation_dir.parent.mkdir(parents=True, exist_ok=True)
        captured: list[Captured] = []

        def writer(staging: Path) -> None:
            captured.append(
                capture(staging, source, source_file, route, request, runner)
            )

        publish_snapshot(request.observation_dir, writer, lock_namespace="native-unit")
        if not captured:
            raise _fail(request, NativeUnitFailure.OUTPUT_INVALID, "capture was empty")
        return _observation(request, captured[0])
    except NativeUnitError:
        raise
    except SnapshotPublishError as error:
        raise _fail(
            request, NativeUnitFailure.OUTPUT_INVALID, "publication failed"
        ) from error
    except (OSError, UnicodeError, ValueError) as error:
        raise _fail(
            request, NativeUnitFailure.OUTPUT_INVALID, "observation failed"
        ) from error


def _observation(request: NativeUnitRequest, captured: Captured) -> NativeObservation:
    directory = request.observation_dir
    execution = directory / "execution.json"
    reference = directory / "reference.pdf"
    info = directory / "pdfinfo.txt"
    return NativeObservation(
        request.source,
        request.run,
        request.nonce,
        captured.unit_count,
        directory,
        execution,
        reference,
        info,
        sha256_file(execution),
        sha256_file(reference),
        sha256_file(info),
    )


def _route(request: NativeUnitRequest) -> NativeRouteSelection:
    route = next(
        (
            route
            for route in request.runtime.routing.routes
            if route.format.value == request.source.document_format.value
        ),
        None,
    )
    if route is None:
        raise _fail(
            request, NativeUnitFailure.SOURCE_INVALID, "routing route is missing"
        )
    commands = route.commands
    document_format = request.source.document_format
    if document_format is DocumentFormat.PDF:
        roles = tuple(command.tool_role for command in commands)
        if roles != (
            ToolRole.POPPLER_METADATA,
            ToolRole.POPPLER_RENDER,
            ToolRole.POPPLER_TEXT,
        ):
            raise _fail(
                request, NativeUnitFailure.SOURCE_INVALID, "PDF route is not exact"
            )
        return NativeRouteSelection(NativeRouteKind.PDF, None, commands[0], commands)
    if document_format in {
        DocumentFormat.DOC,
        DocumentFormat.DOCX,
        DocumentFormat.XLS,
        DocumentFormat.XLSX,
        DocumentFormat.PPT,
        DocumentFormat.PPTX,
    }:
        roles = tuple(command.tool_role for command in commands)
        if roles != (
            ToolRole.LIBREOFFICE,
            ToolRole.POPPLER_METADATA,
            ToolRole.POPPLER_RENDER,
            ToolRole.POPPLER_TEXT,
        ):
            raise _fail(
                request, NativeUnitFailure.SOURCE_INVALID, "office route is not exact"
            )
        return NativeRouteSelection(
            NativeRouteKind.OFFICE, commands[0], commands[1], commands
        )
    raise _fail(request, NativeUnitFailure.SOURCE_INVALID, "format is unsupported")


def _validate(request: NativeUnitRequest) -> None:
    operating_system = platform.system().lower()
    architecture = platform.machine().lower()
    normalized_os = {"darwin": "macos", "linux": "linux"}.get(operating_system)
    normalized_architecture = {
        "aarch64": "arm64",
        "arm64": "arm64",
        "amd64": "x86_64",
        "x86_64": "x86_64",
    }.get(architecture)
    if normalized_os is None or normalized_architecture is None:
        raise NativeUnitError(
            NativeUnitFailure.UNSUPPORTED_PLATFORM,
            None,
            None,
            f"unsupported platform: {operating_system}/{architecture}",
        )
    relative = Path(request.source.relative_path)
    invalid_nonce = len(request.nonce) != 64 or any(
        character not in "0123456789abcdef" for character in request.nonce
    )
    invalid_source = (
        request.run not in (1, 2)
        or invalid_nonce
        or not request.source.source_id
        or not request.source.relative_path
        or relative.is_absolute()
        or request.source.relative_path != relative.as_posix()
        or "\\" in request.source.relative_path
        or any(part in {"", ".", ".."} for part in relative.parts)
    )
    if invalid_source:
        raise _fail(
            request, NativeUnitFailure.SOURCE_INVALID, "source request is invalid"
        )
    if os.path.lexists(request.observation_dir):
        raise _fail(request, NativeUnitFailure.OUTPUT_INVALID, "observation exists")
