from __future__ import annotations

from pathlib import Path

from evaluate.multiformat_evaluator_manifest import validate_evaluator_manifest
from evaluate.multiformat_metric_types import MetricError
from evaluate.multiformat_metrics import validate_metrics_evidence
from evaluate.multiformat_report import acceptance_failures, build_report
from evaluate.multiformat_schema import JsonValue, object_value


def validate_generated_report(
    report: dict[str, JsonValue],
    contract_path: Path,
    contract: dict[str, JsonValue],
    contract_hash: str,
    lock_hash: str,
    evidence_root: Path,
    evaluator_path: Path,
    corpus_path: Path,
    metrics_path: Path,
) -> list[str]:
    try:
        evaluator_hash = validate_evaluator_manifest(
            contract_path.parents[2],
            contract_path,
            evaluator_path,
        )
        summary = validate_metrics_evidence(
            contract_path,
            corpus_path,
            metrics_path,
            evaluator_hash,
            lock_hash,
            evidence_root,
        )
        generated = build_report(
            summary,
            contract_hash,
            lock_hash,
            object_value(report, "evaluator"),
            object_value(report, "corpus_manifest"),
            object_value(report, "metrics_evidence"),
        )
        failures = list(acceptance_failures(summary, contract))
        if generated != report:
            failures.insert(0, "report.aggregate_mismatch")
        return failures
    except MetricError as error:
        return [error.reason]
