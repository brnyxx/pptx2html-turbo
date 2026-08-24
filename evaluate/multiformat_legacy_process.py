from __future__ import annotations

import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Protocol

from evaluate.multiformat_candidate_process import run_bounded_process
from evaluate.multiformat_legacy_types import LegacyConformanceError
from evaluate.multiformat_subprocess import clean_subprocess_environment

MAX_LOG_BYTES = 1024 * 1024


@dataclass(frozen=True, slots=True)
class LegacyProcessRequest:
    command: tuple[str, ...]
    cwd: Path
    environment: dict[str, str]
    stdout_path: Path
    stderr_path: Path
    timeout_seconds: float


class LegacyProcessRunner(Protocol):
    def __call__(self, request: LegacyProcessRequest) -> int: ...


def run_process(request: LegacyProcessRequest) -> int:
    return run_bounded_process(
        request.command,
        request.cwd,
        request.environment,
        request.stdout_path,
        request.stderr_path,
        timeout_seconds=request.timeout_seconds,
        max_log_bytes=MAX_LOG_BYTES,
    )


def run_checked(
    runner: LegacyProcessRunner,
    request: LegacyProcessRequest,
    message: str,
) -> None:
    if runner(request) != 0:
        raise LegacyConformanceError(message)


def tool_version(
    path: Path,
    arguments: tuple[str, ...],
    runner: LegacyProcessRunner,
) -> str:
    with tempfile.TemporaryDirectory(prefix="legacy-tool-version-") as temp_dir:
        root = Path(temp_dir)
        request = LegacyProcessRequest(
            (path.as_posix(), *arguments),
            root,
            {
                **clean_subprocess_environment(),
                "LANG": "C",
                "LC_ALL": "C",
                "TZ": "UTC",
            },
            root / "stdout",
            root / "stderr",
            15.0,
        )
        run_checked(runner, request, "legacy tool version failed")
        value = (
            _read_optional(request.stdout_path) + _read_optional(request.stderr_path)
        ).strip()
        if not value or len(value) > MAX_LOG_BYTES:
            raise LegacyConformanceError("legacy tool version failed")
        return value.splitlines()[0]


def _read_optional(path: Path) -> str:
    if not path.exists():
        return ""
    return path.read_text(encoding="utf-8")
