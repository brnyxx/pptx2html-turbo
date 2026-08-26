from __future__ import annotations

import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import cast

from evaluate.multiformat_candidate_artifacts import (
    evidence_binding,
    write_canonical_json,
)
from evaluate.multiformat_corpus_items import object_list, require_keys
from evaluate.multiformat_evidence import resolve_evidence_path
from evaluate.multiformat_schema import (
    JsonValue,
    boolean_value,
    object_value,
    read_object,
    string_list,
    string_value,
)
from evaluate.multiformat_strict_json import read_strict_object


class CommandEvidenceError(RuntimeError):
    pass


@dataclass(frozen=True, slots=True)
class CommandPlan:
    security: tuple[str, ...]
    quality: dict[str, tuple[str, ...]]
    performance: tuple[str, ...]


def load_command_plan(path: Path) -> CommandPlan:
    values = read_strict_object(path)
    require_keys(values, {"security", "quality", "performance"}, "commands")
    quality = object_value(values, "quality")
    fields = {"tests", "builds", "diagnostics", "contract_checks"}
    require_keys(quality, fields, "commands.quality")
    return CommandPlan(
        tuple(string_list(values, "security")),
        {field: tuple(string_list(quality, field)) for field in sorted(fields)},
        tuple(string_list(values, "performance")),
    )


def run_security_cases(
    plan: CommandPlan,
    corpus_path: Path,
    evidence_root: Path,
    output_dir: Path,
    *,
    project_revision: str,
    evaluator_hash: str,
    corpus_hash: str,
    timeout_seconds: int,
) -> list[JsonValue]:
    tracks = object_value(read_strict_object(corpus_path), "tracks")
    sources = object_list(object_value(tracks, "security"), "items", "security")
    result: list[JsonValue] = []
    for source in sources:
        source_id = string_value(source, "id")
        expected = string_value(source, "expected_outcome")
        source_path = resolve_evidence_path(
            corpus_path.parent,
            string_value(source, "path"),
        )
        substitutions = {
            "source": source_path.as_posix(),
            "source_id": source_id,
            "case_family": string_value(source, "case_family"),
            "expected_outcome": expected,
            "format": string_value(read_object(corpus_path), "format"),
        }
        execution = output_dir / "security" / f"{source_id}.json"
        observed = _run_json_command(
            plan.security,
            substitutions,
            execution.with_suffix(".stdout"),
            execution.with_suffix(".stderr"),
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
            "schema_version": 1,
            "status": "PASS",
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
        write_canonical_json(execution, value)
        result.append(
            {
                "source_id": source_id,
                "execution": evidence_binding(evidence_root, execution),
            }
        )
    return result


def run_quality_commands(
    plan: CommandPlan,
    evidence_root: Path,
    output_dir: Path,
    *,
    bindings: dict[str, str],
    timeout_seconds: int,
) -> dict[str, JsonValue]:
    result: dict[str, JsonValue] = {}
    for command_id, argv in sorted(plan.quality.items()):
        path = output_dir / "quality" / f"{command_id}.json"
        exit_code = _run_command(
            argv,
            bindings,
            path.with_suffix(".stdout"),
            path.with_suffix(".stderr"),
            timeout_seconds,
        )
        value: dict[str, JsonValue] = {
            "schema_version": 1,
            "status": "PASS" if exit_code == 0 else "FAIL",
            "command_id": command_id,
            "exit_code": exit_code,
            "project_revision": bindings["project_revision"],
            "evaluator_manifest_sha256": bindings["evaluator_hash"],
            "corpus_manifest_sha256": bindings["corpus_hash"],
        }
        path.parent.mkdir(parents=True, exist_ok=True)
        write_canonical_json(path, value)
        result[command_id] = evidence_binding(evidence_root, path)
    return result


def run_performance_command(
    plan: CommandPlan,
    evidence_root: Path,
    output_dir: Path,
    *,
    bindings: dict[str, str],
    timeout_seconds: int,
) -> dict[str, JsonValue]:
    path = output_dir / "performance.json"
    exit_code = _run_command(
        plan.performance,
        bindings,
        path.with_suffix(".stdout"),
        path.with_suffix(".stderr"),
        timeout_seconds,
    )
    passed = exit_code == 0
    value: dict[str, JsonValue] = {
        "schema_version": 1,
        "status": "PASS" if passed else "FAIL",
        "within_limits": passed,
        "project_revision": bindings["project_revision"],
        "evaluator_manifest_sha256": bindings["evaluator_hash"],
        "corpus_manifest_sha256": bindings["corpus_hash"],
    }
    path.parent.mkdir(parents=True, exist_ok=True)
    write_canonical_json(path, value)
    return {"evidence": evidence_binding(evidence_root, path)}


def _run_json_command(
    argv: tuple[str, ...],
    substitutions: dict[str, str],
    stdout: Path,
    stderr: Path,
    timeout_seconds: int,
) -> dict[str, JsonValue]:
    exit_code = _run_command(argv, substitutions, stdout, stderr, timeout_seconds)
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
    timeout_seconds: int,
) -> int:
    if not argv:
        raise CommandEvidenceError("command cannot be empty")
    stdout.parent.mkdir(parents=True, exist_ok=True)
    command = tuple(argument.format_map(substitutions) for argument in argv)
    try:
        with stdout.open("wb") as output, stderr.open("wb") as errors:
            completed = subprocess.run(
                command,
                stdin=subprocess.DEVNULL,
                stdout=output,
                stderr=errors,
                check=False,
                timeout=timeout_seconds,
            )
    except (OSError, KeyError, ValueError, subprocess.TimeoutExpired) as error:
        raise CommandEvidenceError(f"command failed: {command[0]}") from error
    return completed.returncode
