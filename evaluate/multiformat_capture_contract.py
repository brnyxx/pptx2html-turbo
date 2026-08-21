from __future__ import annotations

from evaluate.multiformat_metric_types import MetricError
from evaluate.multiformat_schema import JsonValue, string_value


def expected_capture_producer(role: str, document_format: str) -> str:
    if role == "candidate":
        return "document2html-candidate"
    if document_format == "pdf":
        return "locked-pdf-renderer"
    return "windows-office-native"


def capture_counts(value: JsonValue) -> tuple[int, int]:
    if not isinstance(value, list) or any(not isinstance(item, dict) for item in value):
        raise MetricError("metrics.binding.capture", "invalid upstream units")
    source_ids = {string_value(item, "source_id") for item in value}
    return len(source_ids), len(value)
