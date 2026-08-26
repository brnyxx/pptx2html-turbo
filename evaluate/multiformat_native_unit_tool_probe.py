from __future__ import annotations

import tempfile
from pathlib import Path

from evaluate.multiformat_native_unit_files import (
    cleanup_workspace,
    identity,
    tool_identity,
    tool_path,
    verify_tool,
)
from evaluate.multiformat_native_unit_process import (
    environment,
    invoke,
    make_request,
    version,
)
from evaluate.multiformat_native_unit_types import (
    NativeCaptureTool,
    NativeCaptureTools,
    NativeProcessContext,
    NativeProcessRecord,
    NativeProcessRunner,
    NativeProcessSpec,
    NativeStableFile,
    NativeUnitRequest,
    NativeVersionProbe,
)

VERSION_TIMEOUT_SECONDS = 120


def prepare_capture_tools(
    parent: Path,
    request: NativeUnitRequest,
    runner: NativeProcessRunner,
) -> NativeCaptureTools:
    workspace = Path(tempfile.mkdtemp(prefix=".native-tool-probe-", dir=parent))
    workspace_identity = identity(workspace.lstat())
    try:
        logs = workspace / "logs"
        home = workspace / "home"
        temporary = workspace / "tmp"
        for directory in (logs, home, temporary):
            directory.mkdir()
        values = environment(home, temporary, None)
        office = tool_path(request.runtime.soffice, request)
        pdfinfo = tool_path(request.runtime.pdfinfo, request)
        office_identity = tool_identity(office, request)
        pdfinfo_identity = tool_identity(pdfinfo, request)
        libreoffice = _probe(
            office,
            office_identity,
            ("--version",),
            "libreoffice_version",
            logs / "soffice-version",
            workspace,
            values,
            request,
            runner,
        )
        poppler = _probe(
            pdfinfo,
            pdfinfo_identity,
            ("-v",),
            "pdfinfo_version",
            logs / "pdfinfo-version",
            workspace,
            values,
            request,
            runner,
        )
        verify_tool(office, office_identity, request)
        verify_tool(pdfinfo, pdfinfo_identity, request)
        return NativeCaptureTools(libreoffice, poppler)
    finally:
        cleanup_workspace(workspace, workspace_identity, request)


def _probe(
    executable: Path,
    executable_identity: NativeStableFile,
    arguments: tuple[str, ...],
    role: str,
    prefix: Path,
    workspace: Path,
    environment_values: tuple[tuple[str, str], ...],
    request: NativeUnitRequest,
    runner: NativeProcessRunner,
) -> NativeCaptureTool:
    processes: list[NativeProcessRecord] = []
    log = invoke(
        NativeProcessContext(
            runner,
            request,
            make_request(
                NativeProcessSpec(
                    executable,
                    arguments,
                    workspace,
                    environment_values,
                    prefix,
                    timeout=VERSION_TIMEOUT_SECONDS,
                    executable_identity=executable_identity,
                )
            ),
            role,
        ),
        role,
        arguments,
        processes,
    )
    process = processes[0]
    return NativeCaptureTool(
        executable_identity,
        version(log, request),
        NativeVersionProbe(
            process.role,
            process.arguments,
            process.timeout_seconds,
            process.exit_code,
        ),
    )
