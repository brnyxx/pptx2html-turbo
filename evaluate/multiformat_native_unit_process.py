from __future__ import annotations

import os
import re
import stat
from pathlib import Path

from evaluate.multiformat_candidate_fonts import CandidateFontEnvironment
from evaluate.multiformat_candidate_process import (
    CandidateProcessError,
    CandidateProcessFailure,
    run_bounded_process,
)
from evaluate.multiformat_native_unit_files import MAX_LOG_BYTES, fail
from evaluate.multiformat_native_unit_trusted import (
    materialize_binary,
    open_trusted_executable,
)
from evaluate.multiformat_native_unit_types import (
    NativeProcessContext,
    NativeProcessLog,
    NativeProcessRecord,
    NativeProcessRequest,
    NativeProcessSpec,
    NativeRoutePaths,
    NativeUnitFailure,
    NativeUnitRequest,
)
from evaluate.multiformat_subprocess import clean_subprocess_environment

_PAGE_PATTERN = re.compile(r"^Pages:\s+([0-9]+)\s*$", re.MULTILINE)


def run_native_process(request: NativeProcessRequest) -> int:
    if request.executable_identity is None:
        return run_bounded_process(
            request.command,
            request.cwd,
            dict(request.environment),
            request.stdout_path,
            request.stderr_path,
            timeout_seconds=request.timeout_seconds,
            max_log_bytes=request.max_log_bytes,
        )
    trusted = open_trusted_executable(
        Path(request.command[0]), request.executable_identity
    )
    snapshot: Path | None = None
    try:
        if trusted.shell_script:
            command = ("/bin/sh", "-c", ". /dev/stdin", *request.command)
            executable = None
            stdin_fd = trusted.descriptor
        else:
            snapshot = materialize_binary(trusted.content, trusted.path.parent)
            command = request.command
            executable = snapshot
            stdin_fd = None
        return run_bounded_process(
            command,
            request.cwd,
            dict(request.environment),
            request.stdout_path,
            request.stderr_path,
            timeout_seconds=request.timeout_seconds,
            max_log_bytes=request.max_log_bytes,
            executable=executable,
            stdin_fd=stdin_fd,
        )
    finally:
        os.close(trusted.descriptor)
        if snapshot is not None:
            snapshot.unlink(missing_ok=True)


def process(context: NativeProcessContext) -> NativeProcessLog:
    try:
        exit_code = context.runner(context.process)
    except CandidateProcessError as error:
        failure = {
            CandidateProcessFailure.TIMEOUT: NativeUnitFailure.TIMEOUT,
            CandidateProcessFailure.LOG_OVERSIZE: NativeUnitFailure.LOG_OVERSIZE,
            CandidateProcessFailure.EXECUTABLE_UNTRUSTED: NativeUnitFailure.TOOL_MISSING,
        }.get(error.failure, NativeUnitFailure.PROCESS_FAILED)
        raise fail(context.request, failure, f"{context.role}: {error}") from error
    except OSError as error:
        raise fail(
            context.request,
            NativeUnitFailure.PROCESS_FAILED,
            f"{context.role}: process error",
        ) from error
    if type(exit_code) is not int or exit_code != 0:
        raise fail(
            context.request,
            NativeUnitFailure.PROCESS_FAILED,
            f"{context.role}: invalid or nonzero exit {exit_code}",
        )
    bounded(context.process.stdout_path, context.request)
    bounded(context.process.stderr_path, context.request)
    return NativeProcessLog(
        context.role,
        context.process.stdout_path,
        context.process.stderr_path,
        exit_code,
    )


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


def bounded(path: Path, request: NativeUnitRequest) -> None:
    try:
        value = path.lstat()
        if not stat.S_ISREG(value.st_mode):
            raise fail(request, NativeUnitFailure.OUTPUT_INVALID, "log is not regular")
        if value.st_size > MAX_LOG_BYTES:
            raise fail(request, NativeUnitFailure.LOG_OVERSIZE, "log exceeds 1 MiB")
        return
    except FileNotFoundError as error:
        raise fail(
            request, NativeUnitFailure.OUTPUT_INVALID, "log is missing"
        ) from error
    except OSError as error:
        raise fail(
            request, NativeUnitFailure.OUTPUT_INVALID, "log cannot be read"
        ) from error


def version(log: NativeProcessLog, request: NativeUnitRequest) -> str:
    try:
        output = (log.stdout.read_bytes() + log.stderr.read_bytes()).decode("utf-8")
    except (OSError, UnicodeError) as error:
        raise fail(
            request, NativeUnitFailure.OUTPUT_INVALID, "version output is unreadable"
        ) from error
    if "\x00" in output:
        raise fail(
            request, NativeUnitFailure.OUTPUT_INVALID, "version output contains NUL"
        )
    whitespace = " \t\n\r\v\f"
    for raw_line in output.split("\n"):
        line = raw_line.strip(whitespace)
        if line:
            if "\r" in line or "\n" in line:
                raise fail(
                    request,
                    NativeUnitFailure.OUTPUT_INVALID,
                    "version output contains embedded line break",
                )
            return line
    raise fail(request, NativeUnitFailure.OUTPUT_INVALID, "version output is empty")


def pages(content: bytes | Path, request: NativeUnitRequest) -> int:
    try:
        raw = content if isinstance(content, bytes) else content.read_bytes()
        text = raw.decode("utf-8")
        matches: list[str] = _PAGE_PATTERN.findall(text)
    except (OSError, UnicodeError) as error:
        raise fail(
            request, NativeUnitFailure.PAGES_MALFORMED, "pdfinfo output is unreadable"
        ) from error
    if len(matches) != 1 or int(matches[0]) <= 0:
        raise fail(
            request,
            NativeUnitFailure.PAGES_MALFORMED,
            "Pages field is not one positive value",
        )
    return int(matches[0])


def make_request(spec: NativeProcessSpec) -> NativeProcessRequest:
    return NativeProcessRequest(
        (spec.executable.as_posix(), *spec.arguments),
        spec.cwd,
        spec.environment,
        spec.prefix.with_suffix(".stdout"),
        spec.prefix.with_suffix(".stderr"),
        spec.timeout,
        MAX_LOG_BYTES,
        spec.executable_identity,
    )


def environment(
    home: Path, temporary: Path, font: CandidateFontEnvironment | None
) -> tuple[tuple[str, str], ...]:
    values = clean_subprocess_environment()
    values.update(
        {
            "HOME": home.as_posix(),
            "LANG": "en_US.UTF-8",
            "LC_ALL": "en_US.UTF-8",
            "TMPDIR": temporary.as_posix(),
            "TZ": "UTC",
        }
    )
    if font:
        values["FONTCONFIG_FILE"] = font.config_path.as_posix()
    return tuple(sorted(values.items()))


def render(arguments: tuple[str, ...], paths: NativeRoutePaths) -> tuple[str, ...]:
    values = (
        ("{source}", paths.source.as_posix()),
        ("{reference_pdf}", paths.pdf.as_posix()),
        ("{output_dir}", paths.output.as_posix()),
        ("{profile_uri}", paths.profile.as_uri()),
    )
    return tuple(replace(argument, values) for argument in arguments)


def replace(argument: str, values: tuple[tuple[str, str], ...]) -> str:
    for placeholder, value in values:
        argument = argument.replace(placeholder, value)
    return argument
