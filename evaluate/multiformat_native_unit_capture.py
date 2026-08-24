from __future__ import annotations

import tempfile
from dataclasses import dataclass
from pathlib import Path

from evaluate.jcs import canonicalize
from evaluate.multiformat_native_unit_files import (
    cleanup_workspace,
    copy_stable,
    fail,
    font_environment,
    identity,
    output_file,
    stable_file,
    tool_path,
    verify_file,
)
from evaluate.multiformat_native_unit_process import (
    environment,
    make_request,
    pages,
    process,
    render,
    version,
)
from evaluate.multiformat_native_unit_types import (
    NativeExecutionData,
    NativeProcessContext,
    NativeProcessLog,
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
from evaluate.multiformat_schema import sha256_file


@dataclass(frozen=True, slots=True)
class Captured:
    unit_count: int


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
        cleanup_workspace(workspace, workspace_identity)


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
    copy_stable(source, staged, source_file, request)
    pdfinfo = tool_path(request.runtime.pdfinfo, request)
    office = tool_path(request.runtime.soffice, request) if route.office else None
    font = (
        font_environment(request, workspace)
        if route.kind is NativeRouteKind.OFFICE
        else None
    )
    env = environment(folders[3], folders[4], font)
    env_values = dict(env)
    route_timeout = (
        route.office.timeout_seconds
        if route.office is not None
        else route.metadata.timeout_seconds
    )
    processes: list[NativeProcessRecord] = []
    office_version: str | None = None
    if office is not None:
        office_log = invoke(
            NativeProcessContext(
                runner,
                request,
                make_request(
                    NativeProcessSpec(
                        office,
                        ("--version",),
                        workspace,
                        env,
                        folders[2] / "soffice-version",
                        timeout=route_timeout,
                    )
                ),
                "soffice-version",
            ),
            "libreoffice_version",
            ("--version",),
            processes,
        )
        office_version = version(office_log, request)
    version_log = invoke(
        NativeProcessContext(
            runner,
            request,
            make_request(
                NativeProcessSpec(
                    pdfinfo,
                    ("-v",),
                    workspace,
                    env,
                    folders[2] / "pdfinfo-version",
                    timeout=route_timeout,
                )
            ),
            "pdfinfo-version",
        ),
        "pdfinfo_version",
        ("-v",),
        processes,
    )
    pdfinfo_version = version(version_log, request)
    if route.office is None:
        pdf = folders[1] / "source.pdf"
        copy_stable(source, pdf, source_file, request)
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
                        timeout=route_timeout,
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
                    timeout=route_timeout,
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
    count = pages(metadata_log.stdout, request)
    reference = staging / "reference.pdf"
    retained_info = staging / "pdfinfo.txt"
    copy_stable(pdf, reference, output, request)
    info_file = stable_file(
        metadata_log.stdout, request, NativeUnitFailure.OUTPUT_INVALID
    )
    copy_stable(metadata_log.stdout, retained_info, info_file, request)
    data = NativeExecutionData(
        request,
        route.kind,
        source_file.sha256,
        sha256_file(office) if office is not None else None,
        office_version,
        sha256_file(pdfinfo),
        pdfinfo_version,
        font.environment_sha256 if font else None,
        tuple(env_values),
        request.runtime.routing.locale,
        request.runtime.routing.timezone,
        tuple(processes),
        request.nonce,
        count,
        reference,
        retained_info,
    )
    _ = (staging / "execution.json").write_bytes(
        canonicalize(execution_record(data)) + b"\n"
    )
    return Captured(count)


def invoke(
    context: NativeProcessContext,
    role: str,
    arguments: tuple[str, ...],
    processes: list[NativeProcessRecord],
) -> NativeProcessLog:
    log = process(context)
    processes.append(
        NativeProcessRecord(
            role, arguments, context.process.timeout_seconds, log.exit_code
        )
    )
    return log
