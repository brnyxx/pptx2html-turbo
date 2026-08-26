from __future__ import annotations

import os
import subprocess
import sys
from collections.abc import Mapping
from dataclasses import dataclass
from pathlib import Path
from typing import Final

from evaluate.multiformat_candidate_types import CandidateCaptureError
from evaluate.multiformat_evidence import resolve_evidence_path
from evaluate.multiformat_portable_lock import validate_reference_lock
from evaluate.multiformat_schema import (
    JsonValue,
    object_value,
    sha256_file,
    sha256_value,
    string_value,
)
from evaluate.multiformat_strict_json import read_strict_object

_NETWORK_ENDPOINT: Final = "1.1.1.1:443"
_PROBE_TIMEOUT_SECONDS: Final = 5
_ACTIVE_ENV: Final = "PPTX2HTML_CANDIDATE_SANDBOX"
_PROBE_ENV: Final = "PPTX2HTML_SANDBOX_PROBE"
_NETWORK_SCRIPT: Final = (
    "import socket; s=socket.create_connection(('1.1.1.1',443),2); s.close()"
)


class CandidateSandboxError(CandidateCaptureError, ValueError):
    """The candidate process sandbox is absent or does not enforce its policy."""


@dataclass(frozen=True, slots=True)
class CandidateSandbox:
    executable: Path
    profile: Path
    oracle_root: Path
    sentinel: Path

    def executable_binding(self, root: Path) -> dict[str, JsonValue]:
        return _binding(root, self.executable)

    def profile_binding(self, root: Path) -> dict[str, JsonValue]:
        return _binding(root, self.profile)

    def oracle_root_binding(self, root: Path) -> dict[str, JsonValue]:
        resolved_root = root.resolve(strict=True)
        resolved = self.oracle_root.resolve(strict=True)
        if not resolved.is_relative_to(resolved_root):
            raise CandidateSandboxError("candidate oracle root escapes evidence root")
        return {"path": resolved.relative_to(resolved_root).as_posix()}

    def sentinel_binding(self, root: Path) -> dict[str, JsonValue]:
        return _binding(root, self.sentinel)


def resolve_locked_sandbox(lock: dict[str, JsonValue], root: Path) -> tuple[Path, Path]:
    sandbox = object_value(lock, "sandbox")
    return (
        _bound_path(root, object_value(sandbox, "executable")),
        _bound_path(root, object_value(sandbox, "profile")),
    )


def resolve_attested_sandbox(
    values: dict[str, JsonValue], root: Path, executable: Path, profile: Path
) -> CandidateSandbox:
    attested_executable = _bound_path(root, object_value(values, "sandbox_executable"))
    attested_profile = _bound_path(root, object_value(values, "sandbox_profile"))
    if attested_executable != executable.resolve(
        strict=True
    ) or attested_profile != profile.resolve(strict=True):
        raise CandidateSandboxError("candidate sandbox path differs from lock")
    network = object_value(values, "network_probe")
    if network != network_probe_value():
        raise CandidateSandboxError("candidate network probe attestation differs")
    oracle = object_value(values, "oracle_probe")
    if string_value(oracle, "result") != "denied":
        raise CandidateSandboxError("candidate oracle probe attestation differs")
    oracle_root = _bound_directory(root, object_value(oracle, "root"))
    sentinel = _bound_path(root, object_value(oracle, "sentinel"))
    if not sentinel.is_relative_to(oracle_root) or sentinel == oracle_root:
        raise CandidateSandboxError("candidate oracle sentinel escapes oracle root")
    return CandidateSandbox(executable, profile, oracle_root, sentinel)


def observe_network_control() -> None:
    """Require the external endpoint to be reachable before sandboxing."""
    result = _run_probe([sys.executable, "-I", "-c", _NETWORK_SCRIPT], os.environ)
    if result != 0:
        raise CandidateSandboxError("candidate positive network control failed")


def observe_sandbox(
    sandbox: CandidateSandbox, *, require_readable_sentinel: bool = True
) -> None:
    """Run bounded positive and negative controls through the exact sandbox."""
    if require_readable_sentinel:
        try:
            sandbox.sentinel.read_bytes()
        except OSError as error:
            raise CandidateSandboxError(
                "candidate oracle sentinel is not readable before sandboxing"
            ) from error
    _probe(sandbox, "control", "raise SystemExit(0)", expect_denied=False)
    _probe(
        sandbox,
        "network",
        _NETWORK_SCRIPT,
        expect_denied=True,
    )
    _probe(
        sandbox,
        "oracle",
        "import os,pathlib; pathlib.Path(os.environ['ORACLE_SENTINEL']).read_bytes()",
        expect_denied=True,
    )


def enter_locked_sandbox(
    evidence_root: Path,
    lock_path: Path,
    attestation_path: Path,
    module: str,
    argv: list[str],
) -> None:
    """Re-exec a schema-2 candidate entry point under the outer-lock sandbox."""
    lock = read_strict_object(lock_path.resolve(strict=True))
    if lock.get("schema_version") != 2:
        return
    root = evidence_root.resolve(strict=True)
    validate_reference_lock(lock_path, root)
    executable, profile = resolve_locked_sandbox(lock, root)
    values = read_strict_object(attestation_path.resolve(strict=True))
    sandbox = resolve_attested_sandbox(values, root, executable, profile)
    if os.environ.get(_ACTIVE_ENV) == sha256_file(profile):
        return
    command, environment = sandbox_command(
        sandbox, [sys.executable, "-m", module, *argv]
    )
    os.execve(executable, command, environment)


def require_active_sandbox(sandbox: CandidateSandbox) -> None:
    if os.environ.get(_ACTIVE_ENV) != sha256_file(sandbox.profile):
        raise CandidateSandboxError("candidate process is not under the locked sandbox")
    observe_sandbox(sandbox, require_readable_sentinel=False)


def sandbox_command(
    sandbox: CandidateSandbox, command: list[str]
) -> tuple[list[str], dict[str, str]]:
    environment = dict(os.environ)
    environment[_ACTIVE_ENV] = sha256_file(sandbox.profile)
    argv = [
        sandbox.executable.as_posix(),
        "-D",
        f"ORACLE_ROOT={sandbox.oracle_root.as_posix()}",
        "-D",
        f"ORACLE_SENTINEL={sandbox.sentinel.as_posix()}",
        "-f",
        sandbox.profile.as_posix(),
        *command,
    ]
    return argv, environment


def network_probe_value() -> dict[str, JsonValue]:
    return {
        "endpoint": _NETWORK_ENDPOINT,
        "control": "reachable",
        "sandbox": "denied",
    }


def oracle_probe_value(root: Path, sandbox: CandidateSandbox) -> dict[str, JsonValue]:
    return {
        "root": sandbox.oracle_root_binding(root),
        "sentinel": sandbox.sentinel_binding(root),
        "result": "denied",
    }


def _probe(
    sandbox: CandidateSandbox, name: str, script: str, *, expect_denied: bool
) -> None:
    command, environment = sandbox_command(
        sandbox, [sys.executable, "-I", "-c", script]
    )
    environment[_PROBE_ENV] = name
    environment["ORACLE_ROOT"] = sandbox.oracle_root.as_posix()
    environment["ORACLE_SENTINEL"] = sandbox.sentinel.as_posix()
    denied = _run_probe(command, environment) != 0
    if denied != expect_denied:
        raise CandidateSandboxError(f"candidate sandbox probe failed: {name}")


def _run_probe(command: list[str], environment: Mapping[str, str]) -> int:
    try:
        return subprocess.run(
            command,
            env=environment,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            timeout=_PROBE_TIMEOUT_SECONDS,
            check=False,
        ).returncode
    except (OSError, subprocess.TimeoutExpired) as error:
        raise CandidateSandboxError(
            "candidate sandbox probe execution failed"
        ) from error


def _binding(root: Path, path: Path) -> dict[str, JsonValue]:
    root = root.resolve(strict=True)
    resolved = path.resolve(strict=True)
    if not resolved.is_relative_to(root):
        raise CandidateSandboxError("candidate sandbox artifact escapes evidence root")
    return {
        "path": resolved.relative_to(root).as_posix(),
        "sha256": sha256_file(resolved),
    }


def _bound_directory(root: Path, binding: dict[str, JsonValue]) -> Path:
    relative = Path(string_value(binding, "path"))
    if relative.is_absolute() or any(
        part in {"", ".", ".."} for part in relative.parts
    ):
        raise CandidateSandboxError("candidate oracle root path is invalid")
    resolved_root = root.resolve(strict=True)
    path = (resolved_root / relative).resolve(strict=True)
    if not path.is_relative_to(resolved_root) or not path.is_dir():
        raise CandidateSandboxError("candidate oracle root path is invalid")
    return path


def _bound_path(root: Path, binding: dict[str, JsonValue]) -> Path:
    path = resolve_evidence_path(root, string_value(binding, "path"))
    if sha256_file(path) != sha256_value(binding, "sha256"):
        raise CandidateSandboxError("candidate sandbox artifact differs")
    return path
