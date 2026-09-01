from __future__ import annotations

import tempfile
from dataclasses import dataclass
from pathlib import Path

from evaluate.jcs import canonicalize
from evaluate.multiformat_native_unit_files import (
    MAX_LOG_BYTES,
    cleanup_workspace,
    copy_stable,
    fail,
    font_environment,
    identity,
    output_file,
    stable_bytes,
    verify_file,
    verify_tool,
    write_snapshot,
)
from evaluate.multiformat_native_unit_observation_tools import (
    resolve_observation_tools,
)
from evaluate.multiformat_native_unit_process import (
    environment,
    invoke,
    make_request,
    pages,
    render,
)
from evaluate.multiformat_native_unit_types import (
    NativeExecutionData,
    NativeProcessContext,
    NativeProcessRecord,
    NativeProcessRunner,
    NativeProcessSpec,
    NativeRouteKind,
    NativeRoutePaths,
    NativeRouteSelection,
    NativeStableFile,
    NativeUnitFailure,
    NativeUnitRequest,
    execution_record,
)


@dataclass(frozen=True, slots=True)
class Captured:
    unit_count: int
    execution_sha256: str
    reference_pdf_sha256: str
    pdfinfo_sha256: str


def capture(
    staging: Path,
    source: Path,
    source_file: NativeStableFile,
    route: NativeRouteSelection,
    request: NativeUnitRequest,
    runner: NativeProcessRunner,
) -> Captured:
    workspace = Path(tempfile.mkdtemp(prefix=".native-unit-", dir=staging.parent))
    workspace_identity = identity(workspace.lstat())
    try:
        return _workspace(
            staging, workspace, source, source_file, route, request, runner
        )
    finally:
        cleanup_workspace(workspace, workspace_identity, request)


def _workspace(
    staging: Path,
    workspace: Path,
    source: Path,
    source_file: NativeStableFile,
    route: NativeRouteSelection,
    request: NativeUnitRequest,
    runner: NativeProcessRunner,
) -> Captured:
    folders = tuple(
        workspace / name
        for name in ("input", "export", "logs", "home", "tmp", "profile")
    )
    for folder in folders:
        folder.mkdir()
    staged = folders[0] / f"source.{request.source.document_format.value}"
    _ = copy_stable(source, staged, source_file, request)
    font = (
        font_environment(request, workspace)
        if route.kind is NativeRouteKind.OFFICE
        else None
    )
    env = environment(folders[3], folders[4], font)
    env_values = dict(env)
    processes: list[NativeProcessRecord] = []
    tools = resolve_observation_tools(
        workspace,
        folders[2],
        env,
        route,
        request,
        runner,
        processes,
    )
    pdfinfo = tools.pdfinfo
    pdfinfo_file = tools.pdfinfo_identity
    pdfinfo_version = tools.pdfinfo_version
    office = tools.office
    office_file = tools.office_identity
    office_version = tools.office_version
    if route.office is None:
        pdf = folders[1] / "source.pdf"
        _ = copy_stable(source, pdf, source_file, request)
    else:
        if office is None:
            raise fail(
                request, NativeUnitFailure.SOURCE_INVALID, "office tool is missing"
            )
        conversion = render(
            route.office.arguments,
            NativeRoutePaths(staged, folders[1], folders[5], staged),
        )
        _ = invoke(
            NativeProcessContext(
                runner,
                request,
                make_request(
                    NativeProcessSpec(
                        office,
                        conversion,
                        workspace,
                        env,
                        folders[2] / "libreoffice",
                        timeout=route.office.timeout_seconds,
                        executable_identity=office_file,
                    )
                ),
                "libreoffice",
            ),
            "libreoffice",
            route.office.arguments,
            processes,
        )
        pdf = folders[1] / "source.pdf"
    output = output_file(pdf, request)
    metadata = render(
        route.metadata.arguments,
        NativeRoutePaths(pdf, folders[1], folders[5], pdf),
    )
    metadata_log = invoke(
        NativeProcessContext(
            runner,
            request,
            make_request(
                NativeProcessSpec(
                    pdfinfo,
                    metadata,
                    workspace,
                    env,
                    folders[2] / "pdfinfo",
                    timeout=route.metadata.timeout_seconds,
                    executable_identity=pdfinfo_file,
                )
            ),
            "pdfinfo",
        ),
        "poppler_metadata",
        route.metadata.arguments,
        processes,
    )
    verify_file(output, pdf, request)
    verify_file(source_file, source, request)
    _metadata_file, metadata_content = stable_bytes(
        metadata_log.stdout, request, NativeUnitFailure.OUTPUT_INVALID, MAX_LOG_BYTES
    )
    count = pages(metadata_content, request)
    reference = staging / "reference.pdf"
    retained_info = staging / "pdfinfo.txt"
    reference_file = copy_stable(pdf, reference, output, request)
    info_file = write_snapshot(retained_info, metadata_content, request)
    verify_tool(pdfinfo, pdfinfo_file, request)
    if office is not None and office_file is not None:
        verify_tool(office, office_file, request)
    data = NativeExecutionData(
        request,
        route.kind,
        source_file.sha256,
        office_file.sha256 if office_file is not None else None,
        office_version,
        pdfinfo_file.sha256,
        pdfinfo_version,
        font.environment_sha256 if font else None,
        tuple(sorted(env_values)),
        request.runtime.routing.locale,
        "en_US.UTF-8",
        "en_US.UTF-8",
        request.runtime.routing.timezone,
        tuple(processes),
        request.nonce,
        count,
        reference_file.sha256,
        info_file.sha256,
    )
    execution_path = staging / "execution.json"
    execution_file = write_snapshot(
        execution_path, canonicalize(execution_record(data)) + b"\n", request
    )
    verify_file(reference_file, reference, request)
    verify_file(info_file, retained_info, request)
    return Captured(
        count,
        execution_file.sha256,
        reference_file.sha256,
        info_file.sha256,
    )
