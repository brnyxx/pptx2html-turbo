from __future__ import annotations

from decimal import Decimal
from pathlib import Path

from evaluate.multiformat_capture_manifest import validate_capture_manifest
from evaluate.multiformat_command_evidence import load_command_plan
from evaluate.multiformat_corpus import validate_corpus_manifest
from evaluate.multiformat_corpus_items import require_keys
from evaluate.multiformat_corpus_types import CorpusError, CorpusStatus
from evaluate.multiformat_metric_capture_links import validate_metric_capture_links
from evaluate.multiformat_metric_compute import resolve_artifact_binding
from evaluate.multiformat_metric_hard_gates import compute_hard_gates
from evaluate.multiformat_metric_links import load_metric_spec
from evaluate.multiformat_metric_types import (
    MetricError,
    MetricsSummary,
    MetricStatus,
)
from evaluate.multiformat_metric_units import (
    compute_blind,
    compute_conformance,
)
from evaluate.multiformat_revision import current_project_revision
from evaluate.multiformat_schema import (
    JsonValue,
    integer_value,
    number_value,
    object_value,
    sha256_file,
    sha256_value,
    string_value,
)
from evaluate.multiformat_strict_json import StrictJsonError, read_strict_object

PROJECT_ROOT = Path(__file__).resolve().parents[1]


def validate_metrics_evidence(
    contract_path: Path,
    corpus_path: Path,
    metrics_path: Path,
    evaluator_manifest_sha256: str,
    oracle_lock_sha256: str,
    evidence_root: Path | None = None,
    oracle_lock_path: Path | None = None,
) -> MetricsSummary:
    try:
        corpus_validation = validate_corpus_manifest(contract_path, corpus_path)
        if corpus_validation.status is not CorpusStatus.READY:
            raise MetricError("metrics.corpus", "corpus is not READY")
        spec = load_metric_spec(corpus_path)
        metrics = read_strict_object(metrics_path)
        require_keys(
            metrics,
            {
                "schema_version",
                "status",
                "format",
                "bindings",
                "conformance",
                "blind",
                "security",
                "determinism",
                "review",
                "quality",
                "performance",
            },
            "metrics.schema",
        )
        if integer_value(metrics, "schema_version") != 2:
            raise MetricError("metrics.schema", "expected version 2")
        try:
            status = MetricStatus(string_value(metrics, "status"))
        except ValueError as error:
            raise MetricError("metrics.status", "unknown status") from error
        if status is not MetricStatus.READY:
            raise MetricError("metrics.status", status.value)
        if string_value(metrics, "format") != spec.document_format.value:
            raise MetricError("metrics.format", spec.document_format.value)
        root = evidence_root or metrics_path.parent
        bindings = object_value(metrics, "bindings")
        require_keys(
            bindings,
            {
                "contract_sha256",
                "corpus_manifest_sha256",
                "evaluator_manifest_sha256",
                "oracle_lock_sha256",
                "project_revision",
                "oracle_capture",
                "candidate_capture",
                "command_plan",
                "command_plan_sha256",
            },
            "metrics.bindings",
        )
        contract_hash = sha256_file(contract_path)
        corpus_hash = sha256_file(corpus_path)
        project_revision = current_project_revision(PROJECT_ROOT)
        _validate_bindings(
            bindings,
            contract_hash,
            corpus_hash,
            evaluator_manifest_sha256,
            oracle_lock_sha256,
            project_revision,
        )
        oracle_capture = resolve_artifact_binding(
            object_value(bindings, "oracle_capture"),
            root,
            "metrics.binding.oracle_capture",
        )
        candidate_capture = resolve_artifact_binding(
            object_value(bindings, "candidate_capture"),
            root,
            "metrics.binding.candidate_capture",
        )
        command_plan_path = resolve_artifact_binding(
            object_value(bindings, "command_plan"),
            root,
            "metrics.binding.command_plan",
        )
        command_plan = load_command_plan(command_plan_path)
        if sha256_value(bindings, "command_plan_sha256") != command_plan.sha256 or (
            oracle_lock_path is not None
            and command_plan.outer_lock_sha256 != oracle_lock_sha256
        ):
            raise MetricError("metrics.binding.command_plan", "digest")
        oracle_units = validate_capture_manifest(
            oracle_capture,
            "oracle",
            spec,
            contract_hash,
            corpus_hash,
            evaluator_manifest_sha256,
            oracle_lock_sha256,
            project_revision,
            root,
            oracle_lock_path,
        )
        candidate_units = validate_capture_manifest(
            candidate_capture,
            "candidate",
            spec,
            contract_hash,
            corpus_hash,
            evaluator_manifest_sha256,
            oracle_lock_sha256,
            project_revision,
            root,
            oracle_lock_path,
        )
        if candidate_units.determinism_path is None or read_strict_object(
            candidate_units.determinism_path
        ) != object_value(metrics, "determinism"):
            raise MetricError("determinism.binding", "candidate manifest")
        validate_metric_capture_links(metrics, oracle_units, candidate_units)
        thresholds = object_value(read_strict_object(contract_path), "thresholds")
        conformance = compute_conformance(
            object_value(metrics, "conformance"),
            spec,
            root,
        )
        blind = compute_blind(
            object_value(metrics, "blind"),
            spec,
            root,
            Decimal(str(number_value(thresholds, "minimum_blind_file_score"))),
        )
        hard_gates = compute_hard_gates(
            metrics,
            spec,
            root,
            oracle_units,
            candidate_units,
            evaluator_manifest_sha256,
            corpus_hash,
            project_revision,
            command_plan,
        )
        _reject_reused_artifacts(
            conformance.artifact_paths,
            blind.artifact_paths,
            hard_gates.artifact_paths,
            oracle_capture.as_posix(),
            candidate_capture.as_posix(),
        )
        return MetricsSummary(
            status,
            spec.document_format,
            conformance.summary,
            conformance.strata,
            conformance.critical_defects,
            blind.summary,
            blind.accepted_files,
            blind.critical_defects,
            hard_gates.security_cases,
            hard_gates.security_passed,
            hard_gates.determinism,
            hard_gates.reviewer_count,
            hard_gates.review_all_passed,
            hard_gates.quality,
            hard_gates.performance_within_limits,
        )
    except MetricError:
        raise
    except (
        CorpusError,
        StrictJsonError,
        OSError,
        TypeError,
        ValueError,
    ) as error:
        raise MetricError("metrics.schema", metrics_path.as_posix()) from error


def _validate_bindings(
    bindings: dict[str, JsonValue],
    contract_hash: str,
    corpus_hash: str,
    evaluator_hash: str,
    oracle_hash: str,
    project_revision: str,
) -> None:
    expected = {
        "contract_sha256": contract_hash,
        "corpus_manifest_sha256": corpus_hash,
        "evaluator_manifest_sha256": evaluator_hash,
        "oracle_lock_sha256": oracle_hash,
    }
    for field, value in expected.items():
        if sha256_value(bindings, field) != value:
            raise MetricError(f"metrics.{field}", value)
    if string_value(bindings, "project_revision") != project_revision:
        raise MetricError("metrics.project_revision", project_revision)


def _reject_reused_artifacts(*groups: frozenset[str] | str) -> None:
    values: list[str] = []
    for group in groups:
        if isinstance(group, str):
            values.append(group)
        else:
            values.extend(group)
    if len(values) != len(set(values)):
        raise MetricError("artifact.path", "artifact reused across evidence roles")
