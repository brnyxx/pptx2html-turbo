from __future__ import annotations

import hashlib
import os
import re
import secrets
import shutil
import tempfile
from pathlib import Path

from evaluate.jcs import canonicalize
from evaluate.multiformat_candidate_fonts import (
    CandidateFontEnvironment,
    CandidateFontError,
    prepare_font_environment,
)
from evaluate.multiformat_candidate_process import (
    CandidateProcessError,
    run_bounded_process,
)
from evaluate.multiformat_native_unit_types import (
    NativeExecutionData,
    NativeObservation,
    NativeProcessContext,
    NativeProcessLog,
    NativeProcessRequest,
    NativeProcessRunner,
    NativeProcessSpec,
    NativeRoutePaths,
    NativeUnitError,
    NativeUnitFailure,
    NativeUnitRequest,
    execution_record,
)
from evaluate.multiformat_reference_routing import FormatRoute, ToolRole
from evaluate.multiformat_schema import sha256_file
from evaluate.multiformat_subprocess import clean_subprocess_environment

MAX_LOG_BYTES = 1024 * 1024
MAX_PDF_BYTES = 64 * 1024 * 1024
_PAGE_PATTERN = re.compile(r"^Pages:\s+([0-9]+)\s*$", re.MULTILINE)

# fmt: off

def run_native_process(request: NativeProcessRequest) -> int:
    """Run one native command with bounded output and process-group cleanup."""
    return run_bounded_process(request.command, request.cwd, dict(request.environment), request.stdout_path, request.stderr_path, timeout_seconds=request.timeout_seconds, max_log_bytes=request.max_log_bytes)


def capture_native_observation(request: NativeUnitRequest, runner: NativeProcessRunner = run_native_process) -> NativeObservation:
    """Capture one isolated native observation and retain only its evidence files."""
    _validate(request)
    source = _file(request.source.path, request, NativeUnitFailure.SOURCE_MISSING)
    soffice = _tool(request.runtime.soffice, request)
    pdfinfo = _tool(request.runtime.pdfinfo, request)
    source_sha256 = sha256_file(source)
    request.observation_dir.parent.mkdir(parents=True, exist_ok=True)
    workspace = Path(tempfile.mkdtemp(prefix=".native-unit-", dir=request.observation_dir.parent))
    published = False
    try:
        folders = tuple(workspace / name for name in ("input", "export", "logs", "home", "tmp", "profile"))
        for folder in folders:
            _ = folder.mkdir()
        staged = folders[0] / f"source.{request.source.document_format.value}"
        _ = shutil.copyfile(source, staged)
        font = _font(request, workspace)
        environment = _environment(folders[3], folders[4], font.config_path)
        route = _route(request)
        office = next((command for command in route.commands if command.tool_role is ToolRole.LIBREOFFICE), None)
        metadata = next((command for command in route.commands if command.tool_role is ToolRole.POPPLER_METADATA), None)
        if metadata is None:
            raise _fail(request, NativeUnitFailure.SOURCE_INVALID, "metadata route is missing")
        logs: list[NativeProcessLog] = []
        soffice_version: str | None = None
        if office is not None:
            log = _process(NativeProcessContext(runner, request, _request(NativeProcessSpec(soffice, ("--version",), workspace, environment, folders[2] / "soffice-version")), "soffice-version"))
            logs.append(log)
            soffice_version = _version(log, request)
        log = _process(NativeProcessContext(runner, request, _request(NativeProcessSpec(pdfinfo, ("-v",), workspace, environment, folders[2] / "pdfinfo-version")), "pdfinfo-version"))
        logs.append(log)
        pdfinfo_version = _version(log, request)
        if office is None:
            pdf = folders[1] / "source.pdf"
            _ = shutil.copyfile(source, pdf)
        else:
            arguments = _render(
                office.arguments,
                NativeRoutePaths(staged, folders[1], folders[5], staged),
            )
            log = _process(NativeProcessContext(runner, request, _request(NativeProcessSpec(soffice, arguments, staged.parent.parent, environment, folders[2] / "libreoffice", office.timeout_seconds)), "libreoffice"))
            logs.append(log)
            pdf = folders[1] / "source.pdf"
        _output(pdf, request)
        arguments = _render(
            metadata.arguments,
            NativeRoutePaths(pdf, folders[1], folders[5], pdf),
        )
        log = _process(NativeProcessContext(runner, request, _request(NativeProcessSpec(pdfinfo, arguments, workspace, environment, folders[2] / "pdfinfo", metadata.timeout_seconds)), "pdfinfo"))
        logs.append(log)
        unit_count = _pages(log.stdout, request)
        if sha256_file(source) != source_sha256 or sha256_file(staged) != source_sha256:
            raise _fail(request, NativeUnitFailure.SOURCE_INVALID, "source changed during capture")
        if request.observation_dir.exists():
            raise _fail(request, NativeUnitFailure.OUTPUT_INVALID, "observation exists")
        request.observation_dir.mkdir(parents=True)
        reference_pdf = request.observation_dir / "reference.pdf"
        retained_info = request.observation_dir / "pdfinfo.txt"
        _ = shutil.copyfile(pdf, reference_pdf)
        _ = shutil.copyfile(log.stdout, retained_info)
        nonce = secrets.token_hex(32)
        routes = tuple((command.tool_role.value, command.arguments) for command in route.commands if command.tool_role in (ToolRole.LIBREOFFICE, ToolRole.POPPLER_METADATA))
        data = NativeExecutionData(request, source_sha256, routes, sha256_file(soffice), soffice_version, sha256_file(pdfinfo), pdfinfo_version, font.config_path.name, font.environment_sha256, tuple(key for key, _value in environment), tuple(logs), nonce, unit_count, reference_pdf, retained_info)
        execution_path = request.observation_dir / "execution.json"
        _ = execution_path.write_bytes(canonicalize(execution_record(data)) + b"\n")
        published = True
        return NativeObservation(request.source, request.run, nonce, unit_count, request.observation_dir, execution_path, reference_pdf, retained_info, sha256_file(execution_path), sha256_file(reference_pdf), sha256_file(retained_info))
    except NativeUnitError:
        raise
    except (OSError, UnicodeError, ValueError) as error:
        raise _fail(request, NativeUnitFailure.OUTPUT_INVALID, "observation failed") from error
    finally:
        shutil.rmtree(workspace, ignore_errors=True)
        if not published:
            shutil.rmtree(request.observation_dir, ignore_errors=True)


def _request(spec: NativeProcessSpec) -> NativeProcessRequest:
    return NativeProcessRequest((spec.executable.as_posix(), *spec.arguments), spec.cwd, spec.environment, spec.prefix.with_suffix(".stdout"), spec.prefix.with_suffix(".stderr"), spec.timeout, MAX_LOG_BYTES)


def _process(context: NativeProcessContext) -> NativeProcessLog:
    try:
        exit_code = context.runner(context.process)
    except CandidateProcessError as error:
        text = str(error)
        failure = NativeUnitFailure.TIMEOUT if "timeout" in text else NativeUnitFailure.LOG_OVERSIZE if "limit" in text else NativeUnitFailure.PROCESS_FAILED
        raise _fail(context.request, failure, f"{context.role}: {text}") from error
    except OSError as error:
        raise _fail(context.request, NativeUnitFailure.PROCESS_FAILED, f"{context.role}: process error") from error
    if exit_code != 0:
        raise _fail(context.request, NativeUnitFailure.PROCESS_FAILED, f"{context.role}: exit {exit_code}")
    return NativeProcessLog(context.role, context.process.stdout_path, context.process.stderr_path, _bounded(context.process.stdout_path, context.request), _bounded(context.process.stderr_path, context.request))


def _bounded(path: Path, request: NativeUnitRequest) -> str:
    try:
        if path.exists() and path.stat().st_size > MAX_LOG_BYTES:
            raise _fail(request, NativeUnitFailure.LOG_OVERSIZE, "log exceeds 1 MiB")
        return sha256_file(path) if path.exists() else hashlib.sha256(b"").hexdigest()
    except OSError as error:
        raise _fail(request, NativeUnitFailure.OUTPUT_INVALID, "log cannot be read") from error


def _version(log: NativeProcessLog, request: NativeUnitRequest) -> str:
    try:
        output = (log.stdout.read_text(encoding="utf-8") if log.stdout.exists() else "") + (log.stderr.read_text(encoding="utf-8") if log.stderr.exists() else "")
    except (OSError, UnicodeError) as error:
        raise _fail(request, NativeUnitFailure.OUTPUT_INVALID, "version output is unreadable") from error
    lines = tuple(line.strip() for line in output.splitlines() if line.strip())
    if not lines:
        raise _fail(request, NativeUnitFailure.OUTPUT_INVALID, "version output is empty")
    return lines[0]


def _pages(path: Path, request: NativeUnitRequest) -> int:
    try:
        matches: list[str] = _PAGE_PATTERN.findall(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError) as error:
        raise _fail(request, NativeUnitFailure.PAGES_MALFORMED, "pdfinfo output is unreadable") from error
    if len(matches) != 1 or int(matches[0]) <= 0:
        raise _fail(request, NativeUnitFailure.PAGES_MALFORMED, "Pages field is not one positive value")
    return int(matches[0])


def _environment(home: Path, temporary: Path, font_config: Path) -> tuple[tuple[str, str], ...]:
    values = clean_subprocess_environment()
    values.update({"FONTCONFIG_FILE": font_config.as_posix(), "HOME": home.as_posix(), "LANG": "C", "LC_ALL": "C", "TMPDIR": temporary.as_posix(), "TZ": "UTC"})
    return tuple(sorted(values.items()))


def _render(arguments: tuple[str, ...], paths: NativeRoutePaths) -> tuple[str, ...]:
    values = (
        ("{source}", paths.source.as_posix()),
        ("{reference_pdf}", paths.pdf.as_posix()),
        ("{output_dir}", paths.output.as_posix()),
        ("{profile_uri}", paths.profile.as_uri()),
    )
    return tuple(_replace(argument, values) for argument in arguments)


def _replace(argument: str, values: tuple[tuple[str, str], ...]) -> str:
    for placeholder, value in values:
        argument = argument.replace(placeholder, value)
    return argument


def _route(request: NativeUnitRequest) -> FormatRoute:
    for route in request.runtime.routing.routes:
        if route.format.value == request.source.document_format.value:
            return route
    raise _fail(request, NativeUnitFailure.SOURCE_INVALID, "routing route is missing")


def _file(path: Path, request: NativeUnitRequest, failure: NativeUnitFailure) -> Path:
    try:
        if path.is_symlink() or not path.is_file():
            raise _fail(request, failure, "path is not a regular file")
        return path.resolve(strict=True)
    except OSError as error:
        raise _fail(request, failure, "path is missing") from error


def _tool(path: Path, request: NativeUnitRequest) -> Path:
    resolved = _file(path, request, NativeUnitFailure.TOOL_MISSING)
    if not os.access(resolved, os.X_OK):
        raise _fail(request, NativeUnitFailure.TOOL_MISSING, "tool is not executable")
    return resolved


def _output(path: Path, request: NativeUnitRequest) -> None:
    try:
        stat = path.lstat()
        if path.is_symlink() or not path.is_file() or stat.st_size <= 0:
            raise _fail(request, NativeUnitFailure.OUTPUT_MISSING, "PDF output is missing")
        if stat.st_size > MAX_PDF_BYTES:
            raise _fail(request, NativeUnitFailure.OUTPUT_OVERSIZE, "PDF exceeds bound")
    except FileNotFoundError as error:
        raise _fail(request, NativeUnitFailure.OUTPUT_MISSING, "PDF output is missing") from error
    except OSError as error:
        raise _fail(request, NativeUnitFailure.OUTPUT_INVALID, "PDF output is unreadable") from error


def _validate(request: NativeUnitRequest) -> None:
    relative = Path(request.source.relative_path)
    if (
        request.run not in (1, 2)
        or not request.source.source_id
        or not request.source.relative_path
        or relative.is_absolute()
        or request.source.relative_path != relative.as_posix()
        or "\\" in request.source.relative_path
        or any(part in {"", ".", ".."} for part in relative.parts)
    ):
        raise _fail(request, NativeUnitFailure.SOURCE_INVALID, "source request is invalid")
    if request.observation_dir.exists():
        raise _fail(request, NativeUnitFailure.OUTPUT_INVALID, "observation exists")


def _font(request: NativeUnitRequest, workspace: Path) -> CandidateFontEnvironment:
    try:
        return prepare_font_environment(request.runtime.font_bundle, workspace / "font-runtime")
    except (CandidateFontError, OSError, UnicodeError, ValueError) as error:
        raise _fail(request, NativeUnitFailure.FONT_INVALID, "font environment failed") from error


def _fail(request: NativeUnitRequest, failure: NativeUnitFailure, detail: str) -> NativeUnitError:
    return NativeUnitError(failure, request.source.document_format, request.source.source_id, detail)
