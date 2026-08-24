from __future__ import annotations

from dataclasses import dataclass
from enum import StrEnum
from pathlib import Path
from typing import Protocol

from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_reference_routing import RoutedCommand, RoutingIdentity
from evaluate.multiformat_schema import JsonValue, sha256_file


class NativeUnitFailure(StrEnum):
    SOURCE_MISSING = "source-missing"
    SOURCE_INVALID = "source-invalid"
    TOOL_MISSING = "tool-missing"
    FONT_INVALID = "font-invalid"
    PROCESS_FAILED = "process-failed"
    TIMEOUT = "timeout"
    LOG_OVERSIZE = "log-oversize"
    OUTPUT_MISSING = "output-missing"
    OUTPUT_INVALID = "output-invalid"
    OUTPUT_OVERSIZE = "output-oversize"
    PAGES_MALFORMED = "pages-malformed"
    UNSUPPORTED_PLATFORM = "unsupported-platform"


@dataclass(frozen=True, slots=True)
class NativeUnitError(Exception):
    failure: NativeUnitFailure
    document_format: DocumentFormat | None
    source_id: str | None
    detail: str

    def __post_init__(self) -> None:
        scope = ""
        if self.document_format is not None and self.source_id is not None:
            scope = f" for {self.document_format.value}/{self.source_id}"
        Exception.__init__(self, f"{self.failure.value}{scope}: {self.detail}")


@dataclass(frozen=True, slots=True)
class NativeUnitSource:
    source_id: str
    document_format: DocumentFormat
    path: Path
    relative_path: str


@dataclass(frozen=True, slots=True)
class NativeUnitRuntime:
    soffice: Path
    pdfinfo: Path
    font_bundle: Path
    routing: RoutingIdentity


@dataclass(frozen=True, slots=True)
class NativeProcessRequest:
    command: tuple[str, ...]
    cwd: Path
    environment: tuple[tuple[str, str], ...]
    stdout_path: Path
    stderr_path: Path
    timeout_seconds: int
    max_log_bytes: int


class NativeProcessRunner(Protocol):
    def __call__(self, request: NativeProcessRequest) -> int: ...


@dataclass(frozen=True, slots=True)
class NativeUnitRequest:
    source: NativeUnitSource
    runtime: NativeUnitRuntime
    observation_dir: Path
    run: int
    nonce: str


@dataclass(frozen=True, slots=True)
class NativeProcessSpec:
    executable: Path
    arguments: tuple[str, ...]
    cwd: Path
    environment: tuple[tuple[str, str], ...]
    prefix: Path
    timeout: int = 120


@dataclass(frozen=True, slots=True)
class NativeProcessContext:
    runner: NativeProcessRunner
    request: NativeUnitRequest
    process: NativeProcessRequest
    role: str


@dataclass(frozen=True, slots=True)
class NativeRoutePaths:
    source: Path
    output: Path
    profile: Path
    pdf: Path


class NativeRouteKind(StrEnum):
    OFFICE = "office"
    PDF = "pdf"


@dataclass(frozen=True, slots=True)
class NativeRouteSelection:
    kind: NativeRouteKind
    office: RoutedCommand | None
    metadata: RoutedCommand
    commands: tuple[RoutedCommand, ...]


@dataclass(frozen=True, slots=True)
class NativeStableFile:
    device: int
    inode: int
    size: int
    modified_ns: int
    sha256: str


@dataclass(frozen=True, slots=True)
class NativeProcessLog:
    role: str
    stdout: Path
    stderr: Path
    exit_code: int


@dataclass(frozen=True, slots=True)
class NativeProcessRecord:
    role: str
    arguments: tuple[str, ...]
    timeout_seconds: int
    exit_code: int


@dataclass(frozen=True, slots=True)
class NativeExecutionData:
    request: NativeUnitRequest
    route_kind: NativeRouteKind
    source_sha256: str
    soffice_sha256: str | None
    soffice_version: str | None
    pdfinfo_sha256: str
    pdfinfo_version: str
    font_environment_sha256: str | None
    environment_keys: tuple[str, ...]
    environment_locale: str
    environment_lang: str
    environment_lc_all: str
    environment_timezone: str
    processes: tuple[NativeProcessRecord, ...]
    workspace_nonce: str
    unit_count: int
    reference_pdf: Path
    pdfinfo_path: Path


def execution_record(data: NativeExecutionData) -> dict[str, JsonValue]:
    request = data.request
    tools: dict[str, JsonValue] = {
        "pdfinfo": {
            "name": request.runtime.pdfinfo.name,
            "sha256": data.pdfinfo_sha256,
            "version": data.pdfinfo_version,
        }
    }
    environment: dict[str, JsonValue] = {
        "keys": list(data.environment_keys),
        "locale": data.environment_locale,
        "lang": data.environment_lang,
        "lc_all": data.environment_lc_all,
        "timezone": data.environment_timezone,
        "home_isolated": True,
        "temporary_root_isolated": True,
        "profile_isolated": data.route_kind is NativeRouteKind.OFFICE,
    }
    record: dict[str, JsonValue] = {
        "schema_version": 1,
        "source": {
            "id": request.source.source_id,
            "format": request.source.document_format.value,
            "path": request.source.relative_path,
            "sha256": data.source_sha256,
        },
        "run": request.run,
        "workspace_nonce": data.workspace_nonce,
        "routing_sha256": request.runtime.routing.sha256,
        "tools": tools,
        "environment": environment,
        "processes": [
            {
                "role": process.role,
                "arguments": list(process.arguments),
                "timeout_seconds": process.timeout_seconds,
                "exit_code": process.exit_code,
            }
            for process in data.processes
        ],
        "evidence": {
            "reference_pdf": {
                "path": _evidence_path(request, "reference.pdf"),
                "sha256": sha256_file(data.reference_pdf),
            },
            "pdfinfo": {
                "path": _evidence_path(request, "pdfinfo.txt"),
                "sha256": sha256_file(data.pdfinfo_path),
            },
        },
        "unit_count": data.unit_count,
    }
    if data.route_kind is NativeRouteKind.OFFICE:
        environment["font_environment_sha256"] = data.font_environment_sha256
        tools["libreoffice"] = {
            "name": request.runtime.soffice.name,
            "sha256": data.soffice_sha256,
            "version": data.soffice_version,
        }
    return record


@dataclass(frozen=True, slots=True)
class NativeObservation:
    source: NativeUnitSource
    run: int
    workspace_nonce: str
    unit_count: int
    observation_dir: Path
    execution_path: Path
    reference_pdf_path: Path
    pdfinfo_path: Path
    execution_sha256: str
    reference_pdf_sha256: str
    pdfinfo_sha256: str


def _evidence_path(request: NativeUnitRequest, name: str) -> str:
    return (
        f"observations/{request.source.document_format.value}/"
        f"{request.source.source_id}/run-{request.run}/{name}"
    )
