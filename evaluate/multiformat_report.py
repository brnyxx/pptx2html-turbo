from __future__ import annotations

from decimal import Decimal

from evaluate.multiformat_metric_types import (
    MetricsSummary,
    rounded_float,
)
from evaluate.multiformat_schema import (
    JsonValue,
    number_value,
    object_value,
)


def build_report(
    summary: MetricsSummary,
    contract_sha256: str,
    oracle_lock_sha256: str,
    evaluator: dict[str, JsonValue],
    corpus_manifest: dict[str, JsonValue],
    metrics_evidence: dict[str, JsonValue],
) -> dict[str, JsonValue]:
    conformance = summary.conformance.rounded()
    blind = summary.blind.rounded()
    return {
        "schema_version": 2,
        "status": "READY",
        "format": summary.document_format.value,
        "contract_sha256": contract_sha256,
        "oracle_lock_sha256": oracle_lock_sha256,
        "evaluator": evaluator,
        "corpus_manifest": corpus_manifest,
        "metrics_evidence": metrics_evidence,
        "conformance": {
            "score": conformance["score"],
            "visual": conformance["visual"],
            "content": conformance["content"],
            "layout": conformance["layout"],
            "unit_count": summary.conformance.count,
            "minimum_unit_score": conformance["minimum"],
            "critical_defects": summary.conformance_critical_defects,
            "strata": {
                name: rounded_float(value)
                for name, value in sorted(summary.conformance_strata.items())
            },
        },
        "blind": {
            "score": blind["score"],
            "visual": blind["visual"],
            "content": blind["content"],
            "layout": blind["layout"],
            "file_count": summary.blind.count,
            "accepted_files": summary.blind_accepted_files,
            "critical_defects": summary.blind_critical_defects,
            "minimum_file_score": blind["minimum"],
        },
        "security": {
            "case_count": summary.security_cases,
            "passed": summary.security_passed,
        },
        "determinism": {
            "runs": summary.determinism.runs,
            "html_hashes_equal": summary.determinism.html_hashes_equal,
            "inventory_hashes_equal": summary.determinism.inventory_hashes_equal,
            "png_hashes_equal": summary.determinism.png_hashes_equal,
        },
        "review": {
            "reviewers": summary.reviewer_count,
            "all_passed": summary.review_all_passed,
        },
        "quality": {
            "tests_passed": summary.quality.tests_passed,
            "builds_passed": summary.quality.builds_passed,
            "diagnostics_passed": summary.quality.diagnostics_passed,
            "contract_checks_passed": summary.quality.contract_checks_passed,
        },
        "performance": {
            "within_limits": summary.performance_within_limits,
        },
    }


def acceptance_failures(
    summary: MetricsSummary,
    contract: dict[str, JsonValue],
) -> tuple[str, ...]:
    thresholds = object_value(contract, "thresholds")
    failures: list[str] = []
    for prefix, track in [
        ("conformance", summary.conformance),
        ("blind", summary.blind),
    ]:
        for field, value, threshold_name in [
            ("score", track.score, "format_score"),
            ("visual", track.visual, "visual_score"),
            ("content", track.content, "content_score"),
            ("layout", track.layout, "layout_score"),
        ]:
            if value < _threshold(thresholds, threshold_name):
                failures.append(f"{prefix}.{field}")
    stratum_threshold = _threshold(thresholds, "stratum_score")
    if any(value < stratum_threshold for value in summary.conformance_strata.values()):
        failures.append("conformance.strata")
    if summary.conformance.minimum < _threshold(thresholds, "minimum_unit_score"):
        failures.append("conformance.minimum_unit_score")
    if summary.conformance_critical_defects:
        failures.append("conformance.critical_defects")
    if summary.blind.minimum < _threshold(thresholds, "minimum_blind_file_score"):
        failures.append("blind.minimum_file_score")
    if (
        summary.blind_accepted_files != summary.blind.count
        or summary.blind_critical_defects
    ):
        failures.append("blind.critical_defects")
    if summary.security_cases != summary.security_passed:
        failures.append("security")
    if not all(
        [
            summary.determinism.html_hashes_equal,
            summary.determinism.inventory_hashes_equal,
            summary.determinism.png_hashes_equal,
        ]
    ):
        failures.append("determinism")
    if summary.reviewer_count != 2 or not summary.review_all_passed:
        failures.append("review")
    if not summary.quality.all_passed():
        failures.append("quality")
    if not summary.performance_within_limits:
        failures.append("performance")
    return tuple(dict.fromkeys(failures))


def _threshold(values: dict[str, JsonValue], field: str) -> Decimal:
    return Decimal(str(number_value(values, field)))
