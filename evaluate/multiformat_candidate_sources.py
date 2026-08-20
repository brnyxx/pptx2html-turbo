from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

from evaluate.multiformat_corpus import validate_corpus_manifest
from evaluate.multiformat_corpus_items import object_list
from evaluate.multiformat_corpus_types import CorpusStatus, DocumentFormat
from evaluate.multiformat_evidence import resolve_evidence_path
from evaluate.multiformat_metric_types import MetricError
from evaluate.multiformat_schema import (
    JsonValue,
    integer_value,
    object_value,
    sha256_value,
    string_value,
)
from evaluate.multiformat_strict_json import read_strict_object


@dataclass(frozen=True, slots=True)
class CandidateUnitSpec:
    unit_id: str
    ordinal: int


@dataclass(frozen=True, slots=True)
class CandidateSource:
    track: str
    source_id: str
    source_sha256: str
    path: Path
    units: tuple[CandidateUnitSpec, ...]


@dataclass(frozen=True, slots=True)
class CandidateSourceSet:
    document_format: DocumentFormat
    sources: tuple[CandidateSource, ...]


def load_candidate_sources(
    contract_path: Path,
    manifest_path: Path,
) -> CandidateSourceSet:
    validation = validate_corpus_manifest(contract_path, manifest_path)
    if validation.status is not CorpusStatus.READY:
        raise MetricError("candidate.corpus", "manifest is not READY")
    manifest = read_strict_object(manifest_path)
    document_format = DocumentFormat(string_value(manifest, "format"))
    tracks = object_value(manifest, "tracks")
    root = manifest_path.parent.resolve(strict=True)
    result: list[CandidateSource] = []
    source_ids: set[str] = set()
    for track in ["conformance", "blind"]:
        track_value = object_value(tracks, track)
        for source in object_list(track_value, "items", f"{track}.items"):
            source_id = string_value(source, "id")
            if source_id in source_ids:
                raise MetricError("candidate.source_id", source_id)
            source_ids.add(source_id)
            units = (
                _conformance_units(source)
                if track == "conformance"
                else _blind_units(source_id, integer_value(source, "unit_count"))
            )
            result.append(
                CandidateSource(
                    track,
                    source_id,
                    sha256_value(source, "sha256"),
                    resolve_evidence_path(root, string_value(source, "path")),
                    units,
                )
            )
    return CandidateSourceSet(document_format, tuple(result))


def _conformance_units(
    source: dict[str, JsonValue],
) -> tuple[CandidateUnitSpec, ...]:
    return tuple(
        CandidateUnitSpec(
            string_value(unit, "id"),
            integer_value(unit, "ordinal"),
        )
        for unit in object_list(source, "units", "conformance.units")
    )


def _blind_units(
    source_id: str,
    count: int,
) -> tuple[CandidateUnitSpec, ...]:
    return tuple(
        CandidateUnitSpec(f"{source_id}-unit-{ordinal}", ordinal)
        for ordinal in range(1, count + 1)
    )
