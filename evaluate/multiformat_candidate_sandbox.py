from __future__ import annotations

import os
import subprocess
import sys
import tempfile
from collections.abc import Mapping
from dataclasses import dataclass
from pathlib import Path
from typing import Final

from evaluate.multiformat_candidate_sandbox_probe import (
    ActiveSandboxProbeError,
    NETWORK_ENDPOINT,
    NETWORK_SCRIPT,
    ORACLE_SCRIPT,
    UNIX_SOCKET_SCRIPT,
    require_current_process_isolation,
)
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

_PROBE_TIMEOUT_SECONDS: Final = 5
ACTIVE_SANDBOX_ENV: Final = "PPTX2HTML_CANDIDATE_SANDBOX"
_PROBE_ENV: Final = "PPTX2HTML_SANDBOX_PROBE"


class CandidateSandboxError(CandidateCaptureError, ValueError):
    """The candidate process sandbox is absent or does not enforce its policy."""


@dataclass(frozen=True, slots=True)
class CandidateSandbox:
    executable: Path
    profile: Path
    libreoffice: Path
    chromium: Path
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


def resolve_locked_sandbox(
    lock: dict[str, JsonValue], root: Path
) -> tuple[Path, Path, Path, Path]:
    sandbox = object_value(lock, "sandbox")
    tools = object_value(lock, "tools")
    browser = object_value(lock, "browser")
    return (
        _bound_path(root, object_value(sandbox, "executable")),
        _bound_path(root, object_value(sandbox, "profile")),
        _bound_path(root, object_value(tools, "libreoffice")),
        _bound_path(root, object_value(browser, "chromium")),
    )


def resolve_attested_sandbox(
    values: dict[str, JsonValue], root: Path, locked: tuple[Path, Path, Path, Path]
) -> CandidateSandbox:
    executable, profile, libreoffice, chromium = locked
    attested_executable = _bound_path(root, object_value(values, "sandbox_executable"))
    attested_profile = _bound_path(root, object_value(values, "sandbox_profile"))
    locked_paths = (executable.resolve(strict=True), profile.resolve(strict=True))
    if (attested_executable, attested_profile) != locked_paths:
        raise CandidateSandboxError("candidate sandbox path differs from lock")
    network = object_value(values, "network_probe")
    if network != network_probe_value():
        raise CandidateSandboxError("candidate network probe attestation differs")
    oracle = object_value(values, "oracle_probe")
    if string_value(oracle, "result") != "denied":
        raise CandidateSandboxError("candidate oracle probe attestation differs")
    verify_oracle_paths = os.environ.get(ACTIVE_SANDBOX_ENV) != sha256_file(profile)
    sentinel = _bound_path(
        root,
        object_value(oracle, "sentinel"),
        verify_digest=verify_oracle_paths,
        verify_access=verify_oracle_paths,
    )
    oracle_root = _bound_directory(
        root, object_value(oracle, "root"), verify_access=verify_oracle_paths
    )
    if not sentinel.is_relative_to(oracle_root) or sentinel == oracle_root:
        raise CandidateSandboxError("candidate oracle sentinel escapes oracle root")
    return CandidateSandbox(
        executable,
        profile,
        libreoffice,
        chromium,
        oracle_root,
        sentinel,
    )


def observe_network_control() -> None:
    result = _run_probe([sys.executable, "-I", "-c", NETWORK_SCRIPT], os.environ)
    if result != 0:
        raise CandidateSandboxError("candidate positive network control failed")


def observe_sandbox(
    sandbox: CandidateSandbox, *, require_readable_sentinel: bool = True
) -> None:
    if require_readable_sentinel:
        try:
            sandbox.sentinel.read_bytes()
        except OSError as error:
            raise CandidateSandboxError(
                "candidate oracle sentinel is not readable before sandboxing"
            ) from error
    _probe(sandbox, "control", "raise SystemExit(0)", expect_denied=False)
    _probe(sandbox, "network", NETWORK_SCRIPT, expect_denied=True)
    _probe(sandbox, "unix-socket", UNIX_SOCKET_SCRIPT, expect_denied=True)
    _probe(sandbox, "oracle", ORACLE_SCRIPT, expect_denied=True)


def enter_locked_sandbox(
    evidence_root: Path,
    lock_path: Path,
    attestation_path: Path,
    module: str,
    argv: list[str],
) -> None:
    lock = read_strict_object(lock_path.resolve(strict=True))
    if lock.get("schema_version") != 2:
        return
    root = evidence_root.resolve(strict=True)
    validate_reference_lock(lock_path, root)
    locked = resolve_locked_sandbox(lock, root)
    values = read_strict_object(attestation_path.resolve(strict=True))
    sandbox = resolve_attested_sandbox(values, root, locked)
    if os.environ.get(ACTIVE_SANDBOX_ENV) == sha256_file(sandbox.profile):
        _require_current_process_isolation(sandbox)
        return
    command, environment = sandbox_command(
        sandbox, [sys.executable, "-m", module, *argv]
    )
    os.execve(sandbox.executable, command, environment)


def require_active_sandbox(sandbox: CandidateSandbox) -> None:
    if os.environ.get(ACTIVE_SANDBOX_ENV) != sha256_file(sandbox.profile):
        raise CandidateSandboxError("candidate process is not under the locked sandbox")
    _require_current_process_isolation(sandbox)


def _require_current_process_isolation(sandbox: CandidateSandbox) -> None:
    try:
        require_current_process_isolation(
            sandbox.oracle_root, sandbox.sentinel, NETWORK_ENDPOINT
        )
    except ActiveSandboxProbeError as error:
        raise CandidateSandboxError(str(error)) from error


def sandbox_command(
    sandbox: CandidateSandbox, command: list[str]
) -> tuple[list[str], dict[str, str]]:
    environment = dict(os.environ)
    environment[ACTIVE_SANDBOX_ENV] = sha256_file(sandbox.profile)
    environment["TMPDIR"] = tempfile.mkdtemp(
        prefix="pptx2html-chromium-",
        dir="/private/tmp",
    )
    argv = [
        sandbox.executable.as_posix(),
        "-D",
        f"ORACLE_ROOT={sandbox.oracle_root.as_posix()}",
        "-D",
        f"ORACLE_SENTINEL={sandbox.sentinel.as_posix()}",
        "-D",
        f"LIBREOFFICE={sandbox.libreoffice.as_posix()}",
        "-D",
        f"CHROMIUM={sandbox.chromium.as_posix()}",
        "-f",
        sandbox.profile.as_posix(),
        *command,
    ]
    return argv, environment


def network_probe_value() -> dict[str, JsonValue]:
    return {
        "endpoint": NETWORK_ENDPOINT,
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


def _bound_directory(
    root: Path, binding: dict[str, JsonValue], *, verify_access: bool = True
) -> Path:
    resolved_root = root.resolve(strict=True)
    path = _unresolved_bound_path(
        resolved_root, binding, "candidate oracle root path is invalid"
    )
    if not verify_access:
        return path
    path = path.resolve(strict=True)
    if not path.is_relative_to(resolved_root) or not path.is_dir():
        raise CandidateSandboxError("candidate oracle root path is invalid")
    return path


def _bound_path(
    root: Path,
    binding: dict[str, JsonValue],
    *,
    verify_digest: bool = True,
    verify_access: bool = True,
) -> Path:
    if verify_access:
        path = resolve_evidence_path(root, string_value(binding, "path"))
    else:
        path = _unresolved_bound_path(
            root.resolve(strict=True),
            binding,
            "candidate sandbox artifact path is invalid",
        )
    if verify_digest and sha256_file(path) != sha256_value(binding, "sha256"):
        raise CandidateSandboxError("candidate sandbox artifact differs")
    return path


def _unresolved_bound_path(
    root: Path, binding: dict[str, JsonValue], message: str
) -> Path:
    value = string_value(binding, "path")
    relative = Path(value)
    if (
        relative.is_absolute()
        or "\\" in value
        or value != relative.as_posix()
        or any(part in {"", ".", ".."} for part in relative.parts)
    ):
        raise CandidateSandboxError(message)
    return root / relative
