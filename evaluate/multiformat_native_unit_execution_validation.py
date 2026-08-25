from __future__ import annotations

from dataclasses import dataclass

from evaluate.multiformat_corpus_items import object_list, require_keys
from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_native_unit_types import NativeUnitError, NativeUnitFailure
from evaluate.multiformat_reference_routing import RoutingIdentity
from evaluate.multiformat_schema import (
    JsonValue,
    integer_value,
    object_value,
    sha256_value,
    string_list,
    string_value,
)

_OFFICE_KEYS = [
    "FONTCONFIG_FILE",
    "HOME",
    "LANG",
    "LC_ALL",
    "PATH",
    "TMPDIR",
    "TZ",
]
_PDF_KEYS = ["HOME", "LANG", "LC_ALL", "PATH", "TMPDIR", "TZ"]


@dataclass(frozen=True, slots=True)
class NativeExecutionBindings:
    routing: RoutingIdentity
    tools: dict[str, JsonValue]
    font_environment_sha256: str


def validate_execution_record(
    execution: dict[str, JsonValue],
    document_format: DocumentFormat,
    source_id: str,
    relative_path: str,
    source_sha256: str,
    count: int,
    run: int,
    nonce: str,
    parent_bindings: dict[str, dict[str, JsonValue]],
    bindings: NativeExecutionBindings,
) -> None:
    require_keys(
        execution,
        {
            "schema_version",
            "source",
            "run",
            "workspace_nonce",
            "routing_sha256",
            "tools",
            "processes",
            "environment",
            "evidence",
            "unit_count",
        },
        "native.execution",
    )
    source = object_value(execution, "source")
    require_keys(source, {"id", "format", "path", "sha256"}, "source")
    if (
        integer_value(execution, "schema_version") != 1
        or string_value(source, "format") != document_format.value
        or string_value(source, "id") != source_id
        or string_value(source, "path") != relative_path
        or sha256_value(source, "sha256") != source_sha256
        or integer_value(execution, "run") != run
        or string_value(execution, "workspace_nonce") != nonce
        or integer_value(execution, "unit_count") != count
        or sha256_value(execution, "routing_sha256") != bindings.routing.sha256
    ):
        raise _failure("execution parent binding differs")
    _validate_tools(execution, document_format, bindings)
    _validate_processes(execution, document_format, bindings.routing)
    _validate_environment(execution, document_format, bindings)
    evidence = object_value(execution, "evidence")
    require_keys(evidence, {"reference_pdf", "pdfinfo"}, "native.execution.evidence")
    for field in ("reference_pdf", "pdfinfo"):
        if object_value(evidence, field) != parent_bindings[field]:
            raise _failure("execution evidence binding differs")


def _validate_tools(
    execution: dict[str, JsonValue],
    document_format: DocumentFormat,
    bindings: NativeExecutionBindings,
) -> None:
    tools = object_value(execution, "tools")
    expected_keys = (
        {"pdfinfo"}
        if document_format is DocumentFormat.PDF
        else {"libreoffice", "pdfinfo"}
    )
    require_keys(tools, expected_keys, "native.execution.tools")
    for name in expected_keys:
        value = object_value(tools, name)
        require_keys(value, {"name", "sha256", "version"}, "native.execution.tool")
        if value != object_value(bindings.tools, name):
            raise _failure("execution tool binding differs")


def _validate_processes(
    execution: dict[str, JsonValue],
    document_format: DocumentFormat,
    routing: RoutingIdentity,
) -> None:
    route = next(
        item for item in routing.routes if item.format.value == document_format.value
    )
    if document_format is DocumentFormat.PDF:
        expected = (
            ("pdfinfo_version", ["-v"], 120),
            (
                "poppler_metadata",
                list(route.commands[0].arguments),
                route.commands[0].timeout_seconds,
            ),
        )
    else:
        expected = (
            ("libreoffice_version", ["--version"], 120),
            ("pdfinfo_version", ["-v"], 120),
            (
                "libreoffice",
                list(route.commands[0].arguments),
                route.commands[0].timeout_seconds,
            ),
            (
                "poppler_metadata",
                list(route.commands[1].arguments),
                route.commands[1].timeout_seconds,
            ),
        )
    processes = object_list(execution, "processes", "native.execution.processes")
    if len(processes) != len(expected):
        raise _failure("execution process count differs")
    for process, (role, arguments, timeout) in zip(
        processes,
        expected,
        strict=True,
    ):
        require_keys(
            process,
            {"role", "arguments", "timeout_seconds", "exit_code"},
            "native.execution.process",
        )
        if (
            string_value(process, "role") != role
            or string_list(process, "arguments") != arguments
            or integer_value(process, "timeout_seconds") != timeout
            or integer_value(process, "exit_code") != 0
        ):
            raise _failure("execution process binding differs")


def _validate_environment(
    execution: dict[str, JsonValue],
    document_format: DocumentFormat,
    bindings: NativeExecutionBindings,
) -> None:
    environment = object_value(execution, "environment")
    common = {
        "keys",
        "locale",
        "lang",
        "lc_all",
        "timezone",
        "home_isolated",
        "temporary_root_isolated",
        "profile_isolated",
    }
    expected_keys = common | (
        {"font_environment_sha256"}
        if document_format is not DocumentFormat.PDF
        else set[str]()
    )
    require_keys(environment, expected_keys, "native.execution.environment")
    expected_environment_keys = (
        _PDF_KEYS if document_format is DocumentFormat.PDF else _OFFICE_KEYS
    )
    if (
        string_list(environment, "keys") != expected_environment_keys
        or string_value(environment, "locale") != "en-US"
        or string_value(environment, "lang") != "en_US.UTF-8"
        or string_value(environment, "lc_all") != "en_US.UTF-8"
        or string_value(environment, "timezone") != "UTC"
        or environment.get("home_isolated") is not True
        or environment.get("temporary_root_isolated") is not True
        or environment.get("profile_isolated")
        is not (document_format is not DocumentFormat.PDF)
    ):
        raise _failure("execution environment differs")
    if document_format is not DocumentFormat.PDF and (
        sha256_value(environment, "font_environment_sha256")
        != bindings.font_environment_sha256
    ):
        raise _failure("execution font environment differs")


def _failure(detail: str) -> NativeUnitError:
    return NativeUnitError(NativeUnitFailure.OUTPUT_INVALID, None, None, detail)
