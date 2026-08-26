from __future__ import annotations

import platform
from collections.abc import Callable
from pathlib import Path

from evaluate.multiformat_candidate_process import run_bounded_process
from evaluate.multiformat_portable_reference_outputs import executable

MAX_LOG_BYTES = 8 * 1024 * 1024
_SANDBOX_PROFILE = (
    "(version 1)\n(allow default)\n(deny network*)\n"
    "(allow network* (local unix-socket))\n"
    "(allow network* (remote unix-socket))\n"
)


class PortableReferenceProcessError(ValueError):
    pass


class PortableReferenceProcessIncompleteError(PortableReferenceProcessError):
    pass


def run_trusted_process(
    command: tuple[str, ...],
    cwd: Path,
    environment: dict[str, str],
    index: int,
    timeout: int,
    sandbox_exec: Path,
    sandbox_profile: Path,
    verify_runtime: Callable[[], None],
) -> None:
    verify_runtime()
    sandboxed = _sandbox(command, sandbox_exec, sandbox_profile)
    result = run_bounded_process(
        sandboxed,
        cwd,
        environment,
        cwd / f"command-{index}.stdout",
        cwd / f"command-{index}.stderr",
        timeout_seconds=timeout,
        max_log_bytes=MAX_LOG_BYTES,
    )
    if result != 0:
        raise PortableReferenceProcessError(
            f"portable reference command {index} failed"
        )


def _sandbox(
    command: tuple[str, ...], sandbox_exec: Path, sandbox_profile: Path
) -> tuple[str, ...]:
    if platform.system() != "Darwin":
        raise PortableReferenceProcessIncompleteError(
            "portable network sandbox is unsupported on this host"
        )
    sandbox = executable(sandbox_exec)
    profile = sandbox_profile.resolve(strict=True)
    if profile.read_text(encoding="utf-8") != _SANDBOX_PROFILE:
        raise PortableReferenceProcessError("portable sandbox profile differs")
    return sandbox, "-f", profile.as_posix(), *command
