from __future__ import annotations

import hashlib
from dataclasses import dataclass
from enum import StrEnum
from pathlib import Path
from typing import Final, NewType, assert_never

from evaluate.jcs import JcsError, canonicalize
from evaluate import multiformat_reference_routing_schema as schema
from evaluate.multiformat_conformance_pdf import pdf_canonicalizer_identity
from evaluate.multiformat_schema import JsonValue
from evaluate.multiformat_strict_json import StrictJsonError, read_strict_object

RoutingTableSha256 = NewType("RoutingTableSha256", str)

_SCHEMA_VERSION: Final = 1
_CANONICALIZER_VERSION: Final = "1"
_TIMEOUT_SECONDS: Final = 120
_ENVIRONMENT_WHITELIST: Final = ("HOME", "LANG", "LC_ALL", "TZ")
_LOCALE: Final = "en-US"
_TIMEZONE: Final = "UTC"
_OFFICE_ARGUMENTS: Final = (
    "--headless",
    "--nologo",
    "--nodefault",
    "--nolockcheck",
    "--nofirststartwizard",
    "-env:UserInstallation={profile_uri}",
    "--convert-to",
    "pdf",
    "--outdir",
    "{output_dir}",
    "{source}",
)


class DocumentFormat(StrEnum):
    DOC = "doc"
    DOCX = "docx"
    XLS = "xls"
    XLSX = "xlsx"
    PPT = "ppt"
    PPTX = "pptx"
    PDF = "pdf"


class ToolRole(StrEnum):
    LIBREOFFICE = "libreoffice"
    POPPLER_METADATA = "poppler_metadata"
    POPPLER_RENDER = "poppler_render"
    POPPLER_TEXT = "poppler_text"


@dataclass(frozen=True, slots=True)
class RoutingError(ValueError):
    reason: str

    def __str__(self) -> str:
        return self.reason


@dataclass(frozen=True, slots=True)
class RoutedCommand:
    tool_role: ToolRole
    arguments: tuple[str, ...]
    timeout_seconds: int
    output_name: str


@dataclass(frozen=True, slots=True)
class FormatRoute:
    format: DocumentFormat
    normative_input: str
    commands: tuple[RoutedCommand, ...]


@dataclass(frozen=True, slots=True)
class RoutingIdentity:
    schema_version: int
    sha256: RoutingTableSha256
    canonicalizer_version: str
    canonicalizer_implementation_sha256: str
    environment_whitelist: tuple[str, ...]
    locale: str
    timezone: str
    network_isolation: bool
    routes: tuple[FormatRoute, ...]


def load_reference_routing(path: Path) -> RoutingIdentity:
    """Parse the locked routing table and return its canonical identity."""
    try:
        value = read_strict_object(path)
        schema.require_keys(
            value,
            {
                "schema_version",
                "reference_profile",
                "canonicalizer_version",
                "canonicalizer_implementation_sha256",
                "runtime",
                "routes",
            },
            "routing table",
        )
        if schema.integer(value, "schema_version") != _SCHEMA_VERSION:
            raise RoutingError("routing schema version is unsupported")
        if schema.string(value, "reference_profile") != "libreoffice-poppler":
            raise RoutingError("routing reference profile is unsupported")
        canonicalizer = pdf_canonicalizer_identity()
        if schema.string(value, "canonicalizer_version") != canonicalizer.version:
            raise RoutingError("routing canonicalizer version is unsupported")
        if (
            schema.string(value, "canonicalizer_implementation_sha256")
            != canonicalizer.implementation_sha256
        ):
            raise RoutingError("routing canonicalizer implementation differs")
        runtime = schema.mapping(value.get("runtime"), "runtime")
        _validate_runtime(runtime)
        route_values = schema.array(value, "routes")
        expected_formats = tuple(DocumentFormat)
        if len(route_values) != len(expected_formats):
            raise RoutingError("routing table must contain seven routes")
        routes = tuple(
            _parse_route(route_value, expected_format)
            for route_value, expected_format in zip(
                route_values,
                expected_formats,
                strict=True,
            )
        )
        digest = RoutingTableSha256(hashlib.sha256(canonicalize(value)).hexdigest())
        return RoutingIdentity(
            schema_version=_SCHEMA_VERSION,
            sha256=digest,
            canonicalizer_version=_CANONICALIZER_VERSION,
            canonicalizer_implementation_sha256=canonicalizer.implementation_sha256,
            environment_whitelist=_ENVIRONMENT_WHITELIST,
            locale=_LOCALE,
            timezone=_TIMEZONE,
            network_isolation=True,
            routes=routes,
        )
    except RoutingError:
        raise
    except (
        JcsError,
        StrictJsonError,
        OSError,
        TypeError,
        UnicodeError,
        ValueError,
    ) as error:
        raise RoutingError("reference routing table is malformed") from error


def _validate_runtime(runtime: dict[str, JsonValue]) -> None:
    schema.require_keys(
        runtime,
        {"environment_whitelist", "locale", "timezone", "network_isolation"},
        "routing runtime",
    )
    whitelist = schema.string_array(runtime, "environment_whitelist")
    if whitelist != _ENVIRONMENT_WHITELIST:
        raise RoutingError("routing environment whitelist is unsupported")
    if schema.string(runtime, "locale") != _LOCALE:
        raise RoutingError("routing locale is unsupported")
    if schema.string(runtime, "timezone") != _TIMEZONE:
        raise RoutingError("routing timezone is unsupported")
    if not schema.boolean(runtime, "network_isolation"):
        raise RoutingError("routing requires network isolation")


def _parse_route(value: JsonValue, expected_format: DocumentFormat) -> FormatRoute:
    route = schema.mapping(value, "route")
    schema.require_keys(route, {"format", "normative_input", "commands"}, "route")
    if schema.string(route, "format") != expected_format.value:
        raise RoutingError("routing formats are missing, duplicated, or reordered")
    if schema.string(route, "normative_input") != "source":
        raise RoutingError("routing normative input is unsupported")
    match expected_format:
        case (
            DocumentFormat.DOC
            | DocumentFormat.DOCX
            | DocumentFormat.XLS
            | DocumentFormat.XLSX
            | DocumentFormat.PPT
            | DocumentFormat.PPTX
        ):
            pdf_input = "{reference_pdf}"
            conversion = ((ToolRole.LIBREOFFICE, _OFFICE_ARGUMENTS, "reference.pdf"),)
        case DocumentFormat.PDF:
            pdf_input = "{source}"
            conversion = ()
        case unreachable:
            assert_never(unreachable)
    render_arguments = (
        (
            "-png",
            "-scale-to-x",
            "960",
            "-scale-to-y",
            "540",
            pdf_input,
            "{render_prefix}",
        )
        if expected_format in {DocumentFormat.PPT, DocumentFormat.PPTX}
        else ("-png", "-r", "144", pdf_input, "{render_prefix}")
    )
    expected_commands = conversion + (
        (ToolRole.POPPLER_METADATA, (pdf_input,), "pdfinfo.txt"),
        (ToolRole.POPPLER_RENDER, render_arguments, "page"),
        (
            ToolRole.POPPLER_TEXT,
            ("-bbox-layout", "-enc", "UTF-8", pdf_input, "{text_output}"),
            "text-layout.html",
        ),
    )
    commands = schema.array(route, "commands")
    if len(commands) != len(expected_commands):
        raise RoutingError("routing command roles are missing or duplicated")
    parsed = tuple(
        _parse_command(command, expected)
        for command, expected in zip(commands, expected_commands, strict=True)
    )
    return FormatRoute(expected_format, "source", parsed)


def _parse_command(
    value: JsonValue,
    expected: tuple[ToolRole, tuple[str, ...], str],
) -> RoutedCommand:
    command = schema.mapping(value, "command")
    schema.require_keys(
        command,
        {"tool_role", "arguments", "timeout_seconds", "output_name"},
        "routing command",
    )
    role, arguments, output_name = expected
    if schema.string(command, "tool_role") != role.value:
        raise RoutingError("routing command tool role is unsupported")
    if schema.string_array(command, "arguments") != arguments:
        raise RoutingError("routing command arguments are unsupported")
    if schema.integer(command, "timeout_seconds") != _TIMEOUT_SECONDS:
        raise RoutingError("routing command timeout must be bounded")
    if schema.string(command, "output_name") != output_name:
        raise RoutingError("routing command output name is unsupported")
    return RoutedCommand(role, arguments, _TIMEOUT_SECONDS, output_name)
