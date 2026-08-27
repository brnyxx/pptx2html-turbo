from __future__ import annotations

import platform
from collections.abc import Callable
from pathlib import Path

from evaluate.multiformat_candidate_process import run_bounded_process
from evaluate.multiformat_portable_lock_io import sandbox_profile_text
from evaluate.multiformat_portable_reference_outputs import executable

MAX_LOG_BYTES = 8 * 1024 * 1024


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
    libreoffice: Path,
    verify_runtime: Callable[[], None],
) -> None:
    verify_runtime()
    sandboxed = _sandbox(command, sandbox_exec, sandbox_profile, libreoffice)
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
    command: tuple[str, ...],
    sandbox_exec: Path,
    sandbox_profile: Path,
    libreoffice: Path,
) -> tuple[str, ...]:
    if platform.system() != "Darwin":
        raise PortableReferenceProcessIncompleteError(
            "portable network sandbox is unsupported on this host"
        )
    sandbox = executable(sandbox_exec)
    profile = sandbox_profile.resolve(strict=True)
    if profile.read_text(encoding="utf-8") != sandbox_profile_text():
        raise PortableReferenceProcessError("portable sandbox profile differs")
    return (
        sandbox,
        "-D",
        "ORACLE_ROOT=/var/empty",
        "-D",
        f"LIBREOFFICE={executable(libreoffice)}",
        "-f",
        profile.as_posix(),
        *command,
    )
