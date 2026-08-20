from __future__ import annotations

from evaluate.multiformat_corpus_items import object_list
from evaluate.multiformat_capture_types import (
    ArtifactIdentity,
    CaptureManifest,
)
from evaluate.multiformat_metric_types import MetricError
from evaluate.multiformat_schema import (
    JsonValue,
    object_value,
    sha256_value,
    string_value,
)


def validate_metric_capture_links(
    metrics: dict[str, JsonValue],
    oracle: CaptureManifest,
    candidate: CaptureManifest,
) -> None:
    seen: set[str] = set()
    conformance = object_value(metrics, "conformance")
    for record in object_list(
        conformance,
        "units",
        "conformance.units",
    ):
        unit_id = string_value(record, "unit_id")
        _validate_unit(record, unit_id, oracle, candidate, seen)
    blind = object_value(metrics, "blind")
    for file_record in object_list(blind, "files", "blind.files"):
        for record in object_list(file_record, "units", "blind.units"):
            unit_id = string_value(record, "unit_id")
            _validate_unit(record, unit_id, oracle, candidate, seen)
    if seen != set(oracle.units) or seen != set(candidate.units):
        raise MetricError("metrics.binding.capture", "unit set mismatch")


def _validate_unit(
    record: dict[str, JsonValue],
    unit_id: str,
    oracle: CaptureManifest,
    candidate: CaptureManifest,
    seen: set[str],
) -> None:
    if unit_id in seen or unit_id not in oracle.units or unit_id not in candidate.units:
        raise MetricError("metrics.binding.capture", unit_id)
    seen.add(unit_id)
    artifacts = object_value(record, "artifacts")
    expected = {
        "reference_png": oracle.units[unit_id].png,
        "reference_inventory": oracle.units[unit_id].inventory,
        "candidate_png": candidate.units[unit_id].png,
        "candidate_inventory": candidate.units[unit_id].inventory,
    }
    for field, identity in expected.items():
        if _identity(object_value(artifacts, field)) != identity:
            raise MetricError("metrics.binding.capture", f"{unit_id}:{field}")


def _identity(binding: dict[str, JsonValue]) -> ArtifactIdentity:
    return ArtifactIdentity(
        string_value(binding, "path"),
        sha256_value(binding, "sha256"),
    )
