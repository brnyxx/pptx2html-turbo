from __future__ import annotations

import os
import sys
from pathlib import Path
from tempfile import TemporaryDirectory
from typing import NamedTuple, cast

from evaluate import multiformat_candidate_artifacts as artifacts
from evaluate import multiformat_candidate_process as process
from evaluate.jcs import canonicalize
from evaluate.multiformat_corpus_items import object_list, require_keys
from evaluate.multiformat_evidence import resolve_evidence_path
from evaluate.multiformat_schema import (
    JsonValue,
    boolean_value,
    integer_value,
    object_value,
    sha256_file,
    sha256_value,
    string_list,
    string_value,
)
from evaluate.multiformat_strict_json import read_strict_object
from evaluate.multiformat_subprocess import clean_subprocess_environment


class CommandEvidenceError(RuntimeError):
    pass


_QUALITY_ROLES = frozenset({"tests", "builds", "diagnostics", "contract_checks"})
_SECURITY_MODULE = "evaluate.run_multiformat_security_case"
_CONTRACT_MODULE = "evaluate.check_exactness_contract"
_SHELL_NAMES = frozenset({"sh", "bash", "dash", "zsh", "fish", "csh", "tcsh", "ksh"})


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
    executable = Path(argv[0])
    if not executable.is_absolute():
        raise CommandEvidenceError(f"{role} executable is not absolute")
    try:
        resolved = executable.resolve(strict=True)
    except OSError as error:
        raise CommandEvidenceError(f"{role} executable is not real") from error
    if (
        not resolved.is_file()
        or not os.access(resolved, os.X_OK)
        or resolved.name in _SHELL_NAMES
    ):
        raise CommandEvidenceError(f"{role} executable is not allowed")
    canonical_argv = (resolved.as_posix(), *argv[1:])
    invoked_index = _validate_role(role, canonical_argv, resolved)
    executables = [(0, resolved.as_posix(), sha256_file(resolved))]
    if invoked_index is not None:
        try:
            invoked = Path(canonical_argv[invoked_index]).resolve(strict=True)
        except OSError as error:
            raise CommandEvidenceError(
                f"{role} invoked executable is not real"
            ) from error
        if (
            not invoked.is_file()
            or not os.access(invoked, os.X_OK)
            or invoked.name in _SHELL_NAMES
        ):
            raise CommandEvidenceError(f"{role} invoked executable is not allowed")
        canonical_argv = (
            *canonical_argv[:invoked_index],
            invoked.as_posix(),
            *canonical_argv[invoked_index + 1 :],
        )
        executables.append((invoked_index, invoked.as_posix(), sha256_file(invoked)))
    import hashlib

    return CommandIdentity(
        role,
        canonical_argv,
        tuple(executables),
        hashlib.sha256(canonicalize(list(canonical_argv))).hexdigest(),
    )


def _validate_role(role: str, argv: tuple[str, ...], executable: Path) -> int | None:
    if role == "security":
        if executable != Path(sys.executable).resolve(strict=True) or argv[1:3] != (
            "-m",
            _SECURITY_MODULE,
        ):
            raise CommandEvidenceError(
                "security command is not the exact internal entry point"
            )
        return None
    if role == "contract_checks":
        if executable != Path(sys.executable).resolve(strict=True) or argv[1:3] != (
            "-m",
            _CONTRACT_MODULE,
        ):
            raise CommandEvidenceError("contract_checks command is not allowed")
        return None
    if role not in {"tests", "builds", "diagnostics", "performance"}:
        raise CommandEvidenceError(f"unknown command role: {role}")
    if executable != Path("/usr/bin/env").resolve(strict=True):
        raise CommandEvidenceError(f"{role} command must use the allowed env launcher")
    index = 1
    while index < len(argv) and "=" in argv[index]:
        name, _, value = argv[index].partition("=")
        if not name or not value:
            raise CommandEvidenceError(f"{role} environment assignment is invalid")
        index += 1
    expected = {
        "tests": "test",
        "builds": "build",
        "diagnostics": "clippy",
        "performance": "test",
    }[role]
    if (
        index + 1 >= len(argv)
        or not Path(argv[index]).is_absolute()
        or Path(argv[index]).name != "cargo"
        or argv[index + 1] != expected
        or (role == "performance" and "--release" not in argv[index + 2 :])
    ):
        raise CommandEvidenceError(f"{role} cargo command is not allowed")
    return index


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


def run_security_cases(
    plan: CommandPlan,
    corpus_path: Path,
    evidence_root: Path,
    output_dir: Path,
    *,
    project_revision: str,
    evaluator_hash: str,
    corpus_hash: str,
    working_directory: Path,
    timeout_seconds: int,
) -> list[JsonValue]:
    corpus = read_strict_object(corpus_path)
    sources = object_list(
        object_value(object_value(corpus, "tracks"), "security"), "items", "security"
    )
    result: list[JsonValue] = []
    for source in sources:
        source_id = string_value(source, "id")
        expected = string_value(source, "expected_outcome")
        source_path = resolve_evidence_path(
            corpus_path.parent, string_value(source, "path")
        )
        substitutions = {
            "source": source_path.as_posix(),
            "source_id": source_id,
            "case_family": string_value(source, "case_family"),
            "expected_outcome": expected,
            "format": string_value(corpus, "format"),
        }
        execution = output_dir / "security" / f"{source_id}.json"
        observed = _run_json_command(
            plan.security.argv,
            substitutions,
            execution.with_suffix(".stdout"),
            execution.with_suffix(".stderr"),
            working_directory,
            timeout_seconds,
        )
        require_keys(
            observed,
            {
                "observed_outcome",
                "typed_error",
                "network_isolation",
                "external_fetches",
                "active_content_executed",
                "within_limits",
            },
            "security.command",
        )
        typed_error = observed.get("typed_error")
        if typed_error is not None and not isinstance(typed_error, str):
            raise CommandEvidenceError("security typed_error must be text or null")
        value: dict[str, JsonValue] = {
            "schema_version": 2,
            "status": "PASS",
            "command_plan_sha256": plan.sha256,
            "command": command_value(plan.security),
            "source_id": source_id,
            "source_sha256": string_value(source, "sha256"),
            "case_family": substitutions["case_family"],
            "expected_outcome": expected,
            "observed_outcome": string_value(observed, "observed_outcome"),
            "typed_error": typed_error,
            "network_isolation": string_value(observed, "network_isolation"),
            "external_fetches": cast(
                list[JsonValue], string_list(observed, "external_fetches")
            ),
            "active_content_executed": boolean_value(
                observed, "active_content_executed"
            ),
            "within_limits": boolean_value(observed, "within_limits"),
            "project_revision": project_revision,
            "evaluator_manifest_sha256": evaluator_hash,
            "corpus_manifest_sha256": corpus_hash,
        }
        execution.parent.mkdir(parents=True, exist_ok=True)
        artifacts.write_canonical_json(execution, value)
        result.append(
            {
                "source_id": source_id,
                "execution": artifacts.evidence_binding(evidence_root, execution),
            }
        )
    return result


def run_quality_commands(
    plan: CommandPlan,
    evidence_root: Path,
    output_dir: Path,
    *,
    bindings: dict[str, str],
    working_directory: Path,
    timeout_seconds: int,
) -> dict[str, JsonValue]:
    result: dict[str, JsonValue] = {}
    for command_id, command in sorted(plan.quality.items()):
        path = output_dir / "quality" / f"{command_id}.json"
        exit_code = _run_command(
            command.argv,
            bindings,
            path.with_suffix(".stdout"),
            path.with_suffix(".stderr"),
            working_directory,
            timeout_seconds,
        )
        value: dict[str, JsonValue] = {
            "schema_version": 2,
            "status": "PASS" if exit_code == 0 else "FAIL",
            "command_id": command_id,
            "command_plan_sha256": plan.sha256,
            "command": command_value(command),
            "exit_code": exit_code,
            "project_revision": bindings["project_revision"],
            "evaluator_manifest_sha256": bindings["evaluator_hash"],
            "corpus_manifest_sha256": bindings["corpus_hash"],
        }
        path.parent.mkdir(parents=True, exist_ok=True)
        artifacts.write_canonical_json(path, value)
        result[command_id] = artifacts.evidence_binding(evidence_root, path)
    return result


def run_performance_command(
    plan: CommandPlan,
    evidence_root: Path,
    output_dir: Path,
    *,
    bindings: dict[str, str],
    working_directory: Path,
    timeout_seconds: int,
) -> dict[str, JsonValue]:
    path = output_dir / "performance.json"
    exit_code = _run_command(
        plan.performance.argv,
        bindings,
        path.with_suffix(".stdout"),
        path.with_suffix(".stderr"),
        working_directory,
        timeout_seconds,
    )
    passed = exit_code == 0
    value: dict[str, JsonValue] = {
        "schema_version": 2,
        "status": "PASS" if passed else "FAIL",
        "within_limits": passed,
        "command_plan_sha256": plan.sha256,
        "command": command_value(plan.performance),
        "project_revision": bindings["project_revision"],
        "evaluator_manifest_sha256": bindings["evaluator_hash"],
        "corpus_manifest_sha256": bindings["corpus_hash"],
    }
    path.parent.mkdir(parents=True, exist_ok=True)
    artifacts.write_canonical_json(path, value)
    return {"evidence": artifacts.evidence_binding(evidence_root, path)}


def _run_json_command(
    argv: tuple[str, ...],
    substitutions: dict[str, str],
    stdout: Path,
    stderr: Path,
    working_directory: Path,
    timeout_seconds: int,
) -> dict[str, JsonValue]:
    exit_code = _run_command(
        argv, substitutions, stdout, stderr, working_directory, timeout_seconds
    )
    if exit_code != 0:
        raise CommandEvidenceError(f"security command exited {exit_code}")
    try:
        return read_strict_object(stdout)
    except (OSError, TypeError, ValueError) as error:
        raise CommandEvidenceError("security command returned invalid JSON") from error


def _run_command(
    argv: tuple[str, ...],
    substitutions: dict[str, str],
    stdout: Path,
    stderr: Path,
    working_directory: Path,
    timeout_seconds: int,
) -> int:
    if not argv:
        raise CommandEvidenceError("command cannot be empty")
    stdout.parent.mkdir(parents=True, exist_ok=True)
    with TemporaryDirectory(dir=stdout.parent) as environment_name:
        environment_root = Path(environment_name)
        environment = clean_subprocess_environment()
        environment.update(
            {
                "HOME": environment_root.as_posix(),
                "TMPDIR": environment_root.as_posix(),
                "TEMP": environment_root.as_posix(),
                "TMP": environment_root.as_posix(),
                "TZ": "UTC",
                "LANG": "C.UTF-8",
                "LC_ALL": "C.UTF-8",
            }
        )
        command = tuple(argument.format_map(substitutions) for argument in argv)
        try:
            return process.run_bounded_process(
                command,
                working_directory,
                environment,
                stdout,
                stderr,
                timeout_seconds=timeout_seconds,
                max_log_bytes=8 * 1024 * 1024,
            )
        except (process.CandidateProcessError, OSError, KeyError, ValueError) as error:
            raise CommandEvidenceError(f"command failed: {command[0]}") from error
