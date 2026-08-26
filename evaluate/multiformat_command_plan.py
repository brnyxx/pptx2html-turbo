from __future__ import annotations

import hashlib
import os
import sys
from pathlib import Path
from typing import NamedTuple

from evaluate.jcs import canonicalize
from evaluate.multiformat_corpus_items import object_list, require_keys
from evaluate.multiformat_schema import (
    JsonValue,
    integer_value,
    object_value,
    sha256_file,
    sha256_value,
    string_list,
    string_value,
)
from evaluate.multiformat_strict_json import read_strict_object


class CommandEvidenceError(RuntimeError):
    pass


_QUALITY_ROLES = frozenset({"tests", "builds", "diagnostics", "contract_checks"})
_SECURITY_MODULE = "evaluate.run_multiformat_security_case"
_CONTRACT_MODULE = "evaluate.check_exactness_contract"
_SHELL_NAMES = frozenset({"sh", "bash", "dash", "zsh", "fish", "csh", "tcsh", "ksh"})
_CARGO_ARGUMENTS = {
    "tests": ("test", "-p", "document2html-core", "-p", "document2html-native"),
    "builds": ("build", "--release", "-p", "pptx2html-cli", "--bin", "document2html"),
    "diagnostics": (
        "clippy",
        "-p",
        "document2html-core",
        "-p",
        "document2html-native",
        "--all-targets",
        "--",
        "-D",
        "warnings",
    ),
    "performance": ("test", "--release", "-p", "document2html-native"),
}


class CommandIdentity(NamedTuple):
    role: str
    argv: tuple[str, ...]
    executables: tuple[tuple[int, str, str], ...]
    argv_sha256: str


class CommandPlan(NamedTuple):
    path: Path
    sha256: str
    security: CommandIdentity
    quality: dict[str, CommandIdentity]
    performance: CommandIdentity


def command_identity(role: str, argv: tuple[str, ...]) -> CommandIdentity:
    if not argv or any(not value for value in argv):
        raise CommandEvidenceError(f"{role} argv is empty")
    executable = _executable(Path(argv[0]), role)
    canonical = (executable.as_posix(), *argv[1:])
    invoked_index = _validate_role(role, canonical, executable)
    executables = [(0, executable.as_posix(), sha256_file(executable))]
    if invoked_index is not None:
        invoked = _executable(Path(canonical[invoked_index]), f"{role} invoked")
        canonical = (
            *canonical[:invoked_index],
            invoked.as_posix(),
            *canonical[invoked_index + 1 :],
        )
        executables.append((invoked_index, invoked.as_posix(), sha256_file(invoked)))
    return CommandIdentity(
        role,
        canonical,
        tuple(executables),
        hashlib.sha256(canonicalize(list(canonical))).hexdigest(),
    )


def _executable(path: Path, label: str) -> Path:
    if not path.is_absolute():
        raise CommandEvidenceError(f"{label} executable is not absolute")
    try:
        resolved = path.resolve(strict=True)
    except OSError as error:
        raise CommandEvidenceError(f"{label} executable is not real") from error
    if (
        not resolved.is_file()
        or not os.access(resolved, os.X_OK)
        or resolved.name in _SHELL_NAMES
    ):
        raise CommandEvidenceError(f"{label} executable is not allowed")
    return resolved


def _validate_role(role: str, argv: tuple[str, ...], executable: Path) -> int | None:
    python = Path(sys.executable).resolve(strict=True)
    if role == "security":
        if executable != python or argv[1:3] != ("-m", _SECURITY_MODULE):
            raise CommandEvidenceError(
                "security command is not the exact internal entry point"
            )
        return None
    if role == "contract_checks":
        if (
            executable != python
            or len(argv) != 5
            or argv[1:4] != ("-m", _CONTRACT_MODULE, "--repo-root")
            or not Path(argv[4]).is_absolute()
        ):
            raise CommandEvidenceError("contract_checks command is not allowed")
        return None
    expected = _CARGO_ARGUMENTS.get(role)
    if expected is None:
        raise CommandEvidenceError(f"unknown command role: {role}")
    if executable != Path("/usr/bin/env").resolve(strict=True):
        raise CommandEvidenceError(f"{role} command must use the allowed env launcher")
    if (
        len(argv) != len(expected) + 3
        or not argv[1].startswith("PATH=")
        or not argv[1].removeprefix("PATH=")
        or "=" in argv[2]
        or not Path(argv[2]).is_absolute()
        or Path(argv[2]).name != "cargo"
        or argv[3:] != expected
    ):
        raise CommandEvidenceError(f"{role} cargo command is not allowed")
    return 2


def command_value(command: CommandIdentity) -> dict[str, JsonValue]:
    return {
        "role": command.role,
        "argv": list(command.argv),
        "argv_sha256": command.argv_sha256,
        "executables": [
            {"argv_index": index, "path": path, "sha256": digest}
            for index, path, digest in command.executables
        ],
    }


def load_command_plan(path: Path) -> CommandPlan:
    values = read_strict_object(path)
    require_keys(
        values, {"schema_version", "security", "quality", "performance"}, "commands"
    )
    if integer_value(values, "schema_version") != 2:
        raise CommandEvidenceError("unsupported command plan schema")
    security = _load_identity(object_value(values, "security"), "security")
    quality_value = object_value(values, "quality")
    require_keys(quality_value, set(_QUALITY_ROLES), "commands.quality")
    quality = {
        role: _load_identity(object_value(quality_value, role), role)
        for role in sorted(_QUALITY_ROLES)
    }
    performance = _load_identity(object_value(values, "performance"), "performance")
    return CommandPlan(
        path.resolve(strict=True), sha256_file(path), security, quality, performance
    )


def _load_identity(values: dict[str, JsonValue], role: str) -> CommandIdentity:
    require_keys(
        values, {"role", "argv", "argv_sha256", "executables"}, f"commands.{role}"
    )
    if string_value(values, "role") != role:
        raise CommandEvidenceError(f"command role differs: {role}")
    identity = command_identity(role, tuple(string_list(values, "argv")))
    if (
        sha256_value(values, "argv_sha256") != identity.argv_sha256
        or object_list(values, "executables", f"commands.{role}.executables")
        != command_value(identity)["executables"]
    ):
        raise CommandEvidenceError(f"command identity differs: {role}")
    return identity
