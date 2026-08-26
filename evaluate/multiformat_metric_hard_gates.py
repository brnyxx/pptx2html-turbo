from __future__ import annotations

from pathlib import Path

from evaluate.multiformat_capture_types import CaptureManifest
from evaluate.multiformat_command_evidence import CommandPlan
from evaluate.multiformat_corpus_types import CorpusError
from evaluate.multiformat_metric_determinism import compute_determinism
from evaluate.multiformat_metric_quality import compute_quality
from evaluate.multiformat_metric_review import compute_review
from evaluate.multiformat_metric_security import compute_security
from evaluate.multiformat_metric_types import (
    CorpusMetricSpec,
    HardGateSummary,
    MetricError,
)
from evaluate.multiformat_schema import (
    JsonValue,
    object_value,
)


def compute_hard_gates(
    metrics: dict[str, JsonValue],
    spec: CorpusMetricSpec,
    evidence_root: Path,
    oracle_capture: CaptureManifest,
    candidate_capture: CaptureManifest,
    evaluator_hash: str,
    corpus_hash: str,
    project_revision: str,
    command_plan: CommandPlan,
) -> HardGateSummary:
    try:
        security_count, security_passed, execution_paths = compute_security(
            object_value(metrics, "security"),
            spec,
            evidence_root,
            evaluator_hash,
            corpus_hash,
            project_revision,
            command_plan,
        )
        determinism, determinism_paths = compute_determinism(
            object_value(metrics, "determinism"),
            spec,
            evidence_root,
            candidate_capture,
        )
        reviewer_count, review_all_passed, review_paths = compute_review(
            object_value(metrics, "review"),
            spec,
            evidence_root,
            oracle_capture,
            candidate_capture,
            object_value(metrics, "bindings"),
        )
        quality, performance, quality_paths = compute_quality(
            object_value(metrics, "quality"),
            object_value(metrics, "performance"),
            evidence_root,
            evaluator_hash,
            corpus_hash,
            project_revision,
            command_plan,
        )
        return HardGateSummary(
            security_count,
            security_passed,
            determinism,
            reviewer_count,
            review_all_passed,
            quality,
            performance,
            frozenset(
                path.as_posix()
                for path in (
                    execution_paths | determinism_paths | review_paths | quality_paths
                )
            ),
        )
    except MetricError:
        raise
    except (CorpusError, TypeError, ValueError) as error:
        raise MetricError("metrics.hard_gates", "invalid evidence") from error
