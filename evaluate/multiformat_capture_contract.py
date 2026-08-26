from __future__ import annotations

from evaluate.multiformat_metric_types import MetricError
from evaluate.multiformat_reference_profile import ReferenceProfile
from evaluate.multiformat_schema import JsonValue, string_value


def expected_capture_producer(
    role: str,
    document_format: str,
    profile: ReferenceProfile | None = None,
) -> str:
    if role == "candidate":
        return "document2html-candidate"
    if profile is ReferenceProfile.LIBREOFFICE_POPPLER:
        return ReferenceProfile.LIBREOFFICE_POPPLER.value
    if document_format == "pdf":
        return "locked-pdf-renderer"
    return "windows-office-native"


def capture_counts(value: JsonValue) -> tuple[int, int]:
    if not isinstance(value, list) or any(not isinstance(item, dict) for item in value):
        raise MetricError("metrics.binding.capture", "invalid upstream units")
    source_ids: set[str] = set()
    for item in value:
        if not isinstance(item, dict):
            raise MetricError("metrics.binding.capture", "invalid upstream units")
        source_ids.add(string_value(item, "source_id"))
    return len(source_ids), len(value)
