from __future__ import annotations

import platform
import subprocess
from pathlib import Path

from evaluate.multiformat_candidate_types import (
    CandidateCaptureError,
    CandidateRuntimePaths,
)
from evaluate.multiformat_schema import (
    JsonValue,
    integer_value,
    sha256_file,
    sha256_value,
    string_value,
)


class CandidateRuntimeLockError(CandidateCaptureError):
    pass


def require_browser_lock(values: dict[str, JsonValue]) -> None:
    expected_numbers = {
        "viewport_width": 1920,
        "viewport_height": 2400,
        "device_scale_factor": 1,
    }
    for field, value in expected_numbers.items():
        if integer_value(values, field) != value:
            raise CandidateRuntimeLockError(f"browser lock mismatch: {field}")
    expected_strings = {
        "locale": "en-US",
        "timezone": "UTC",
        "color_profile": "srgb",
        "reduced_motion": "reduce",
        "animations": "disabled",
        "os": platform.system(),
        "architecture": platform.machine(),
    }
    for field, value in expected_strings.items():
        if string_value(values, field) != value:
            raise CandidateRuntimeLockError(f"browser lock mismatch: {field}")


def validate_candidate_runtime(
    lock: dict[str, JsonValue],
    runtime: CandidateRuntimePaths,
    revision: str,
) -> dict[str, str]:
    if string_value(lock, "build_revision") != revision:
        raise CandidateRuntimeLockError("converter build revision mismatch")
    tools = {
        "converter": (runtime.converter, ("--version",)),
        "soffice": (runtime.soffice, ("--version",)),
        "pdftohtml": (runtime.pdftohtml, ("-v",)),
        "pdfinfo": (runtime.pdfinfo, ("-v",)),
        "receipt_signer": (runtime.receipt_signer, ("--version",)),
    }
    versions: dict[str, str] = {}
    for name, (path, arguments) in tools.items():
        if sha256_file(path) != sha256_value(lock, f"{name}_sha256"):
            raise CandidateRuntimeLockError(f"{name} executable hash mismatch")
        version = _tool_version(path, arguments)
        if version != string_value(lock, f"{name}_version"):
            raise CandidateRuntimeLockError(f"{name} version mismatch")
        versions[f"{name}_version"] = version
    return versions


def require_clean_worktree(
    project_root: Path,
    allowed_evidence_root: Path,
) -> None:
    command = ["git", "status", "--porcelain", "--untracked-files=all", "--", "."]
    if allowed_evidence_root.is_relative_to(project_root):
        command.append(
            f":(exclude){allowed_evidence_root.relative_to(project_root).as_posix()}"
        )
    command.append(":(exclude).omo/senpi-task")
    try:
        result = subprocess.run(
            command,
            cwd=project_root,
            check=True,
            capture_output=True,
            text=True,
            timeout=10,
        )
    except (OSError, subprocess.SubprocessError) as error:
        raise CandidateRuntimeLockError("cannot inspect worktree") from error
    if result.stdout:
        raise CandidateRuntimeLockError("candidate capture requires a clean worktree")


def _tool_version(path: Path, arguments: tuple[str, ...]) -> str:
    try:
        result = subprocess.run(
            [path.as_posix(), *arguments],
            check=True,
            capture_output=True,
            text=True,
            env={"PATH": "/usr/bin:/bin", "LANG": "C", "LC_ALL": "C"},
            timeout=15,
        )
    except (OSError, subprocess.SubprocessError) as error:
        raise CandidateRuntimeLockError(
            f"cannot inspect runtime version: {path}"
        ) from error
    lines = [
        line.strip()
        for line in f"{result.stdout}\n{result.stderr}".splitlines()
        if line.strip()
    ]
    if not lines:
        raise CandidateRuntimeLockError(f"runtime version is empty: {path}")
    return lines[0]
