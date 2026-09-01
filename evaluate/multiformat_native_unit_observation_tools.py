from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

from evaluate.multiformat_native_unit_files import fail, tool_identity, tool_path
from evaluate.multiformat_native_unit_process import (
    invoke,
    make_request,
    version,
)
from evaluate.multiformat_native_unit_types import (
    NativeProcessContext,
    NativeProcessRecord,
    NativeProcessRunner,
    NativeProcessSpec,
    NativeRouteSelection,
    NativeStableFile,
    NativeUnitFailure,
    NativeUnitRequest,
)

VERSION_TIMEOUT_SECONDS = 120


@dataclass(frozen=True, slots=True)
class ObservationTools:
    pdfinfo: Path
    pdfinfo_identity: NativeStableFile
    pdfinfo_version: str
    office: Path | None
    office_identity: NativeStableFile | None
    office_version: str | None


def resolve_observation_tools(
    workspace: Path,
    logs: Path,
    environment: tuple[tuple[str, str], ...],
    route: NativeRouteSelection,
    request: NativeUnitRequest,
    runner: NativeProcessRunner,
    processes: list[NativeProcessRecord],
) -> ObservationTools:
    prepared = request.runtime.tools
    pdfinfo = tool_path(request.runtime.pdfinfo, request)
    pdfinfo_identity = (
        prepared.pdfinfo.identity
        if prepared is not None
        else tool_identity(pdfinfo, request)
    )
    office = tool_path(request.runtime.soffice, request) if route.office else None
    office_identity = (
        prepared.libreoffice.identity
        if prepared is not None and office is not None
        else tool_identity(office, request)
        if office is not None
        else None
    )
    office_version = (
        prepared.libreoffice.version
        if prepared is not None and office is not None
        else None
    )
    if office is not None and prepared is None:
        if office_identity is None:
            raise fail(
                request,
                NativeUnitFailure.SOURCE_INVALID,
                "office tool identity is missing",
            )
        office_version = _probe(
            office,
            office_identity,
            ("--version",),
            "libreoffice_version",
            logs / "soffice-version",
            workspace,
            environment,
            request,
            runner,
            processes,
        )
    pdfinfo_version = (
        prepared.pdfinfo.version
        if prepared is not None
        else _probe(
            pdfinfo,
            pdfinfo_identity,
            ("-v",),
            "pdfinfo_version",
            logs / "pdfinfo-version",
            workspace,
            environment,
            request,
            runner,
            processes,
        )
    )
    return ObservationTools(
        pdfinfo,
        pdfinfo_identity,
        pdfinfo_version,
        office,
        office_identity,
        office_version,
    )


def _probe(
    executable: Path,
    executable_identity: NativeStableFile,
    arguments: tuple[str, ...],
    role: str,
    prefix: Path,
    workspace: Path,
    environment: tuple[tuple[str, str], ...],
    request: NativeUnitRequest,
    runner: NativeProcessRunner,
    processes: list[NativeProcessRecord],
) -> str:
    log = invoke(
        NativeProcessContext(
            runner,
            request,
            make_request(
                NativeProcessSpec(
                    executable,
                    arguments,
                    workspace,
                    environment,
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
    return version(log, request)
