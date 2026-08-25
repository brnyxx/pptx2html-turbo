from __future__ import annotations

import platform
import re
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import cast

from evaluate.multiformat_candidate_process import (
    CandidateProcessError,
    run_bounded_process,
)
from evaluate.multiformat_corpus_items import require_keys
from evaluate.multiformat_native_unit_stable_validation import (
    StableFile,
    stable_bytes,
    stable_file,
)
from evaluate.multiformat_native_unit_types import NativeUnitError, NativeUnitFailure
from evaluate.multiformat_schema import (
    JsonValue,
    integer_value,
    object_value,
    sha256_value,
    string_list,
    string_value,
)
from evaluate.multiformat_subprocess import clean_subprocess_environment

_MAX_OUTPUT = 1024 * 1024
_MAX_TOOL_BYTES = 512 * 1024 * 1024
_MAX_PDF_BYTES = 64 * 1024 * 1024
_PAGES = re.compile(rb"^Pages:\s+([0-9]+)\s*$", re.MULTILINE)


@dataclass(frozen=True, slots=True)
class LockedTool:
    device: int
    inode: int
    size: int
    modified_ns: int
    changed_ns: int
    sha256: str
    version: str


def validate_platform() -> tuple[str, str]:
    operating_system = {"darwin": "macos", "linux": "linux"}.get(
        platform.system().lower()
    )
    architecture = {
        "aarch64": "arm64",
        "arm64": "arm64",
        "amd64": "x86_64",
        "x86_64": "x86_64",
    }.get(platform.machine().lower())
    if operating_system is None or architecture is None:
        raise NativeUnitError(
            NativeUnitFailure.UNSUPPORTED_PLATFORM,
            None,
            None,
            "unsupported platform or architecture",
        )
    return operating_system, architecture


def validate_runtime_bindings(
    values: dict[str, JsonValue],
    libreoffice: Path,
    pdfinfo: Path,
    operating_system: str,
    architecture: str,
) -> LockedTool:
    tools = object_value(values, "tools")
    require_keys(tools, {"libreoffice", "pdfinfo"}, "native.inventory.tools")
    _ = _validate_tool(libreoffice, object_value(tools, "libreoffice"), ("--version",))
    locked_pdfinfo = _validate_tool(
        pdfinfo,
        object_value(tools, "pdfinfo"),
        ("-v",),
    )
    runtime = object_value(values, "runtime")
    require_keys(
        runtime,
        {
            "os",
            "architecture",
            "locale",
            "lang",
            "lc_all",
            "timezone",
            "worker_count",
            "environment_keys",
        },
        "native.inventory.runtime",
    )
    if (
        string_value(runtime, "os") != operating_system
        or string_value(runtime, "architecture") != architecture
        or string_value(runtime, "locale") != "en-US"
        or string_value(runtime, "lang") != "en_US.UTF-8"
        or string_value(runtime, "lc_all") != "en_US.UTF-8"
        or string_value(runtime, "timezone") != "UTC"
        or not 1 <= integer_value(runtime, "worker_count") <= 8
    ):
        raise _failure("inventory runtime identity differs")
    environment = object_value(runtime, "environment_keys")
    require_keys(environment, {"office", "pdf"}, "runtime.environment_keys")
    if string_list(environment, "office") != [
        "FONTCONFIG_FILE",
        "HOME",
        "LANG",
        "LC_ALL",
        "PATH",
        "TMPDIR",
        "TZ",
    ] or string_list(environment, "pdf") != [
        "HOME",
        "LANG",
        "LC_ALL",
        "PATH",
        "TMPDIR",
        "TZ",
    ]:
        raise _failure("inventory environment keys differ")
    return locked_pdfinfo


def validate_pdf_count(
    pdfinfo: Path,
    expected_tool: LockedTool,
    pdf: Path,
    expected_count: int,
) -> None:
    before, content = stable_bytes(
        pdf,
        executable=False,
        maximum=_MAX_PDF_BYTES,
    )
    if not content.startswith(b"%PDF-"):
        raise _failure("retained PDF is malformed")
    _require_same_tool(
        expected_tool,
        stable_file(pdfinfo, executable=True, maximum=_MAX_TOOL_BYTES),
    )
    stdout, _stderr = _execute(pdfinfo, (pdf.as_posix(),))
    matches = _PAGES.findall(stdout)
    page_count = int(cast(bytes, matches[0])) if len(matches) == 1 else 0
    if len(matches) != 1 or page_count != expected_count:
        raise _failure("retained PDF page count differs")
    if stable_file(pdf, executable=False, maximum=_MAX_PDF_BYTES) != before:
        raise _failure("retained PDF changed during validation")
    _require_same_tool(
        expected_tool,
        stable_file(pdfinfo, executable=True, maximum=_MAX_TOOL_BYTES),
    )


def _validate_tool(
    path: Path,
    record: dict[str, JsonValue],
    arguments: tuple[str, ...],
) -> LockedTool:
    require_keys(record, {"name", "sha256", "version"}, "native.inventory.tool")
    before = stable_file(path, executable=True, maximum=_MAX_TOOL_BYTES)
    stdout, stderr = _execute(path, arguments)
    version = _version(stdout + stderr)
    after = stable_file(path, executable=True, maximum=_MAX_TOOL_BYTES)
    if before != after:
        raise _failure("tool changed during validation")
    if (
        string_value(record, "name") != path.name
        or sha256_value(record, "sha256") != before[-1]
        or string_value(record, "version") != version
    ):
        raise _failure("tool binding differs")
    return LockedTool(*before, version)


def _execute(path: Path, arguments: tuple[str, ...]) -> tuple[bytes, bytes]:
    try:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            stdout = root / "stdout"
            stderr = root / "stderr"
            code = run_bounded_process(
                (path.as_posix(), *arguments),
                root,
                clean_subprocess_environment(),
                stdout,
                stderr,
                timeout_seconds=120,
                max_log_bytes=_MAX_OUTPUT,
            )
            if code != 0:
                raise _failure("tool process failed")
            return stdout.read_bytes(), stderr.read_bytes()
    except NativeUnitError:
        raise
    except (CandidateProcessError, OSError, ValueError) as error:
        raise _failure("tool process failed") from error


def _require_same_tool(
    expected: LockedTool,
    actual: StableFile,
) -> None:
    if actual != (
        expected.device,
        expected.inode,
        expected.size,
        expected.modified_ns,
        expected.changed_ns,
        expected.sha256,
    ):
        raise _failure("pdfinfo changed during validation")


def _version(output: bytes) -> str:
    try:
        lines = output.decode("utf-8").split("\n")
    except UnicodeError as error:
        raise _failure("tool version is not UTF-8") from error
    values = [line.strip(" \t\r\v\f") for line in lines if line.strip()]
    if not values or "\x00" in values[0] or "\r" in values[0] or "\n" in values[0]:
        raise _failure("tool version is invalid")
    return values[0]


def _failure(detail: str) -> NativeUnitError:
    return NativeUnitError(NativeUnitFailure.OUTPUT_INVALID, None, None, detail)
