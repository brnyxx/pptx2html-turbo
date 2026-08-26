from __future__ import annotations

from pathlib import Path

from evaluate.multiformat_corpus import validate_corpus_manifest
from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_ready_assembly_types import ReadyValidationError
from evaluate.multiformat_ready_types import (
    ReadyBlind,
    ReadyConformance,
    ReadySecurity,
    ReadySource,
    ReadySourceSet,
    ReadySupport,
)
from evaluate.multiformat_ready_validation_schema import read_canonical_object
from evaluate.multiformat_schema import JsonValue, sha256_file


def validate_copied_inputs(root: Path, source_plan: Path, inventory_root: Path) -> None:
    pairs = (
        (source_plan, root / "conformance-plan.json"),
        (
            inventory_root / "native-unit-inventory.json",
            root / "native-unit-inventory.json",
        ),
    )
    for source, copied in pairs:
        if source.read_bytes() != copied.read_bytes():
            raise ReadyValidationError(f"copied input differs: {copied.name}")


def validate_corpora(
    contract: Path,
    root: Path,
    sources: ReadySourceSet,
) -> tuple[set[str], dict[str, str]]:
    expected_paths = {"conformance-plan.json", "native-unit-inventory.json"}
    expected_digests = {
        "conformance-plan.json": sha256_file(root / "conformance-plan.json"),
        "native-unit-inventory.json": sha256_file(root / "native-unit-inventory.json"),
    }
    support_map = {
        (item.owner_format, item.owner_source_id): item for item in sources.supports
    }
    for document_format in DocumentFormat:
        relative = f"corpora/{document_format.value}/manifest.json"
        manifest_path = root / relative
        manifest = read_canonical_object(manifest_path)
        validation = validate_corpus_manifest(contract, manifest_path)
        if validation.document_format is not document_format:
            raise ReadyValidationError("corpus format binding")
        expected_paths.add(relative)
        expected_digests[relative] = sha256_file(manifest_path)
        tracks = manifest.get("tracks")
        if not isinstance(tracks, dict):
            raise ReadyValidationError("corpus tracks")
        actual = _indexed_items(tracks)
        selected = [
            item for item in sources.sources if item.document_format is document_format
        ]
        if len(actual) != 185 or len(selected) != 185:
            raise ReadyValidationError("corpus source count")
        for source in selected:
            track = _track(source)
            key = track, source.source_id
            item = actual.pop(key, None)
            expected = _expected_item(
                source, support_map.get((document_format, source.source_id))
            )
            if item != expected:
                raise ReadyValidationError(
                    f"corpus item differs: {document_format.value}/{source.source_id}"
                )
            path = expected.get("path")
            if not isinstance(path, str):
                raise ReadyValidationError("corpus item path")
            full_relative = f"corpora/{document_format.value}/{path}"
            _bind_file(root, full_relative, source.source_sha256)
            expected_paths.add(full_relative)
            expected_digests[full_relative] = source.source_sha256
        if actual:
            raise ReadyValidationError("unexpected corpus item")
    return expected_paths, expected_digests


def _indexed_items(
    tracks: dict[str, JsonValue],
) -> dict[tuple[str, str], dict[str, JsonValue]]:
    result: dict[tuple[str, str], dict[str, JsonValue]] = {}
    if set(tracks) != {"conformance", "blind", "security"}:
        raise ReadyValidationError("corpus track fields")
    for track_name, track in tracks.items():
        if not isinstance(track, dict) or set(track) != {"expected_count", "items"}:
            raise ReadyValidationError("corpus track schema")
        items = track.get("items")
        if not isinstance(items, list) or track.get("expected_count") != len(items):
            raise ReadyValidationError("corpus track count")
        for item in items:
            if not isinstance(item, dict):
                raise ReadyValidationError("corpus item schema")
            source_id = item.get("id")
            if not isinstance(source_id, str):
                raise ReadyValidationError("corpus item schema")
            key = track_name, source_id
            if key in result:
                raise ReadyValidationError("duplicate corpus item")
            result[key] = item
    return result


def _expected_item(
    source: ReadySource, support: ReadySupport | None
) -> dict[str, JsonValue]:
    details = source.details
    path = f"sources/{_track(source)}/{source.source_id}.{source.document_format.value}"
    if isinstance(details, ReadyBlind):
        return {
            "id": source.source_id,
            "path": path,
            "sha256": source.source_sha256,
            "producer": details.producer,
            "source_uri": details.source_uri,
            "template_family": details.template_family,
            "unit_count": source.unit_count,
            "applicable_metrics": list(details.applicable_metrics),
            "background": "#ffffff",
        }
    if isinstance(details, ReadySecurity):
        return {
            "id": source.source_id,
            "path": path,
            "sha256": source.source_sha256,
            "case_family": details.case_family,
            "expected_outcome": details.expected_outcome.value,
        }
    paired: JsonValue = None
    if details.support_id is not None:
        if not isinstance(support, ReadySupport):
            raise ReadyValidationError("missing support")
        paired = {
            "id": support.support_id,
            "path": f"sources/support/{support.filename}",
            "sha256": support.source_sha256,
        }
    provenance: JsonValue = None
    if details.provenance is not None:
        provenance = {
            "producer": details.provenance.producer,
            "source_uri": details.provenance.source_uri,
            "independently_authored": details.provenance.independently_authored,
        }
    unit: JsonValue = {
        "id": source.source_id,
        "ordinal": 1,
        "primary_stratum": details.primary_stratum,
        "paired_stratum": details.paired_stratum,
        "applicable_metrics": ["visual", "content", "layout"],
        "background": "#ffffff",
        "secondary_features": [details.feature_seed],
    }
    return {
        "id": source.source_id,
        "path": path,
        "sha256": source.source_sha256,
        "paired_source": paired,
        "provenance": provenance,
        "units": [unit],
    }


def _track(source: ReadySource) -> str:
    if isinstance(source.details, ReadyConformance):
        return "conformance"
    if isinstance(source.details, ReadyBlind):
        return "blind"
    return "security"


def _bind_file(root: Path, relative: str, digest: str) -> None:
    path = root / relative
    if sha256_file(path) != digest:
        raise ReadyValidationError(f"source digest differs: {relative}")
