from __future__ import annotations

from evaluate.multiformat_schema import (
    JsonValue,
    boolean_value,
    integer_value,
    number_value,
    object_value,
    string_list,
)


def check_track(
    prefix: str,
    track: dict[str, JsonValue],
    thresholds: dict[str, JsonValue],
    failures: list[str],
) -> None:
    for field, threshold in [
        ("score", "format_score"),
        ("visual", "visual_score"),
        ("content", "content_score"),
        ("layout", "layout_score"),
    ]:
        minimum(track, field, thresholds, threshold, f"{prefix}.{field}", failures)


def check_strata(
    document_format: str,
    conformance: dict[str, JsonValue],
    contract: dict[str, JsonValue],
    thresholds: dict[str, JsonValue],
    failures: list[str],
) -> None:
    expected = set(string_list(object_value(contract, "strata"), document_format))
    actual = object_value(conformance, "strata")
    if set(actual) != expected:
        failures.append("conformance.strata")
        return
    threshold = number_value(thresholds, "stratum_score")
    if any(number_value(actual, name) < threshold for name in sorted(expected)):
        failures.append("conformance.strata")


def check_hard_gates(
    report: dict[str, JsonValue],
    corpus: dict[str, JsonValue],
    failures: list[str],
) -> None:
    security = object_value(report, "security")
    expected_cases = integer_value(corpus, "security_cases")
    if (
        integer_value(security, "case_count") != expected_cases
        or integer_value(security, "passed") != expected_cases
    ):
        failures.append("security")
    determinism = object_value(report, "determinism")
    if integer_value(determinism, "runs") != integer_value(
        corpus, "deterministic_runs"
    ):
        failures.append("determinism")
    for field in ["html_hashes_equal", "inventory_hashes_equal", "png_hashes_equal"]:
        if not boolean_value(determinism, field):
            failures.append(f"determinism.{field}")
    review = object_value(report, "review")
    if integer_value(review, "reviewers") != integer_value(
        corpus, "reviewers"
    ) or not boolean_value(review, "all_passed"):
        failures.append("review")


def minimum(
    values: dict[str, JsonValue],
    field: str,
    thresholds: dict[str, JsonValue],
    threshold_field: str,
    reason: str,
    failures: list[str],
) -> None:
    if number_value(values, field) < number_value(thresholds, threshold_field):
        failures.append(reason)
