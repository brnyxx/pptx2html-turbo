"""Binding and artifact-reuse checks for metrics evidence.

Extracted from `multiformat_metrics` so that module holds only the validation
sequence itself.
"""

from __future__ import annotations

from evaluate.multiformat_metric_types import MetricError
from evaluate.multiformat_schema import JsonValue, sha256_value, string_value


def validate_bindings(
    bindings: dict[str, JsonValue],
    expected: dict[str, str],
    project_revision: str,
) -> None:
    """Reject metrics whose declared bindings do not match the computed ones."""
    for field, value in expected.items():
        if sha256_value(bindings, field) != value:
            raise MetricError(f"metrics.{field}", value)
    if string_value(bindings, "project_revision") != project_revision:
        raise MetricError("metrics.project_revision", project_revision)


def reject_reused_artifacts(*groups: frozenset[str] | str) -> None:
    """Reject evidence that cites one artifact in more than one role."""
    values: list[str] = []
    for group in groups:
        if isinstance(group, str):
            values.append(group)
        else:
            values.extend(group)
    if len(values) != len(set(values)):
        raise MetricError("artifact.path", "artifact reused across evidence roles")
