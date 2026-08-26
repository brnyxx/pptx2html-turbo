from __future__ import annotations

from pathlib import Path
from tempfile import TemporaryDirectory
from typing import cast

from evaluate import multiformat_candidate_artifacts as artifacts
from evaluate import multiformat_candidate_process as process
from evaluate.multiformat_command_plan import (
    CommandEvidenceError,
    CommandIdentity,
    CommandPlan,
    command_value,
    revalidate_command_identity,
)
from evaluate.multiformat_corpus_items import object_list, require_keys
from evaluate.multiformat_evidence import resolve_evidence_path
from evaluate.multiformat_schema import (
    JsonValue,
    boolean_value,
    object_value,
    string_list,
    string_value,
)
from evaluate.multiformat_strict_json import read_strict_object
from evaluate.multiformat_subprocess import clean_subprocess_environment


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
            identity=command,
            plan=plan,
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
        identity=plan.performance,
        plan=plan,
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
    *,
    identity: CommandIdentity | None = None,
    plan: CommandPlan | None = None,
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
        if identity is not None and plan is not None:
            revalidate_command_identity(identity, plan)
        try:
            exit_code = process.run_bounded_process(
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
        finally:
            if identity is not None and plan is not None:
                revalidate_command_identity(identity, plan)
        return exit_code
