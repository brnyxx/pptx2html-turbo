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


@dataclass(frozen=True, slots=True)
class NativeUnitError(Exception):
    failure: NativeUnitFailure
    document_format: DocumentFormat
    source_id: str
    detail: str

    def __post_init__(self) -> None:
        Exception.__init__(
            self,
            f"{self.failure.value} for {self.document_format.value}/{self.source_id}: {self.detail}",
        )


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
    timeout_seconds: float
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
    timeout: float = 120.0


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
    stdout_sha256: str
    stderr_sha256: str


@dataclass(frozen=True, slots=True)
class NativeExecutionData:
    request: NativeUnitRequest
    route_kind: NativeRouteKind
    source_sha256: str
    routes: tuple[tuple[str, tuple[str, ...]], ...]
    soffice_sha256: str | None
    soffice_version: str | None
    pdfinfo_sha256: str
    pdfinfo_version: str
    font_config_name: str | None
    font_environment_sha256: str | None
    environment_keys: tuple[str, ...]
    environment_locale: str
    environment_timezone: str
    logs: tuple[NativeProcessLog, ...]
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
        "timezone": data.environment_timezone,
        "home_isolated": True,
        "temporary_root_isolated": True,
        "profile_isolated": True,
    }
    record: dict[str, JsonValue] = {
        "schema_version": 1,
        "source_id": request.source.source_id,
        "format": request.source.document_format.value,
        "source": {"path": request.source.relative_path, "sha256": data.source_sha256},
        "run": request.run,
        "workspace_nonce": data.workspace_nonce,
        "routing_sha256": request.runtime.routing.sha256,
        "routes": [
            {"tool_role": role, "arguments": list(arguments)}
            for role, arguments in data.routes
        ],
        "tools": tools,
        "environment": environment,
        "logs": [
            {
                "role": log.role,
                "exit_code": log.exit_code,
                "stdout_sha256": log.stdout_sha256,
                "stderr_sha256": log.stderr_sha256,
            }
            for log in data.logs
        ],
        "evidence": {
            "reference_pdf": {
                "path": data.reference_pdf.name,
                "sha256": sha256_file(data.reference_pdf),
            },
            "pdfinfo": {
                "path": data.pdfinfo_path.name,
                "sha256": sha256_file(data.pdfinfo_path),
            },
        },
        "unit_count": data.unit_count,
    }
    if data.route_kind is NativeRouteKind.OFFICE:
        environment["font_environment_sha256"] = data.font_environment_sha256
        tools["soffice"] = {
            "name": request.runtime.soffice.name,
            "sha256": data.soffice_sha256,
            "version": data.soffice_version,
        }
        record["font"] = {
            "config": data.font_config_name,
            "environment_sha256": data.font_environment_sha256,
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


@dataclass(frozen=True, slots=True)
class NativeUnitSummary:
    source: NativeUnitSource
    unit_count: int
    observations: tuple[NativeObservation, ...]
