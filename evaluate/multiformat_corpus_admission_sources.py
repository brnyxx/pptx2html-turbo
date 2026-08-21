from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Final

from evaluate.multiformat_corpus import validate_corpus_manifest
from evaluate.multiformat_corpus_admission_types import AdmissionPlan, AdmissionSource
from evaluate.multiformat_corpus_items import object_list
from evaluate.multiformat_corpus_sources import resolve_source_path
from evaluate.multiformat_corpus_types import CorpusError, CorpusStatus, DocumentFormat
from evaluate.multiformat_schema import (
    JsonValue,
    integer_value,
    object_value,
    sha256_file,
    string_value,
)
from evaluate.multiformat_strict_json import read_strict_object

_FORMATS: Final = frozenset(DocumentFormat)


@dataclass(frozen=True, slots=True)
class _SourceContext:
    root: Path
    document_format: DocumentFormat
    track: str
    unit_count: int


def load_admission_sources(plan: AdmissionPlan) -> tuple[AdmissionSource, ...]:
    """Validate seven legacy manifests and collect every frozen source."""
    validations = tuple(
        validate_corpus_manifest(plan.contract_path, path)
        for path in plan.corpus_manifests
    )
    formats = {validation.document_format for validation in validations}
    if len(validations) != len(_FORMATS) or formats != _FORMATS:
        raise CorpusError("admission.formats", "exactly one of all seven required")
    if any(validation.status is not CorpusStatus.READY for validation in validations):
        raise CorpusError("admission.status", "all input corpora must be READY")
    sources: list[AdmissionSource] = []
    identities: set[tuple[DocumentFormat, str]] = set()
    for manifest_path in plan.corpus_manifests:
        manifest = read_strict_object(manifest_path)
        document_format = DocumentFormat(string_value(manifest, "format"))
        tracks = object_value(manifest, "tracks")
        for track in ("conformance", "blind", "security"):
            for item in object_list(
                object_value(tracks, track), "items", f"{track}.items"
            ):
                source = _admission_source(
                    item,
                    _SourceContext(
                        manifest_path.parent,
                        document_format,
                        track,
                        _unit_count(item, track),
                    ),
                )
                _add_source(sources, identities, source)
                paired = item.get("paired_source")
                if track == "conformance" and isinstance(paired, dict):
                    _add_source(
                        sources,
                        identities,
                        _admission_source(
                            paired,
                            _SourceContext(
                                manifest_path.parent,
                                _paired_format(document_format),
                                "support",
                                0,
                            ),
                        ),
                    )
    return tuple(sources)


def _admission_source(
    item: dict[str, JsonValue],
    context: _SourceContext,
) -> AdmissionSource:
    path = resolve_source_path(context.root, string_value(item, "path"))
    return AdmissionSource(
        document_format=context.document_format,
        item_id=string_value(item, "id"),
        track=context.track,
        path=path,
        digest=sha256_file(path),
        unit_count=context.unit_count,
    )


def _unit_count(item: dict[str, JsonValue], track: str) -> int:
    if track == "conformance":
        return len(object_list(item, "units", "conformance.units"))
    if track == "blind":
        return integer_value(item, "unit_count")
    return 1


def _add_source(
    sources: list[AdmissionSource],
    identities: set[tuple[DocumentFormat, str]],
    source: AdmissionSource,
) -> None:
    identity = (source.document_format, source.item_id)
    if identity in identities:
        raise CorpusError(
            "source.id", f"{source.document_format.value}:{source.item_id}"
        )
    identities.add(identity)
    sources.append(source)


def _paired_format(document_format: DocumentFormat) -> DocumentFormat:
    pairs = {
        DocumentFormat.DOC: DocumentFormat.DOCX,
        DocumentFormat.XLS: DocumentFormat.XLSX,
        DocumentFormat.PPT: DocumentFormat.PPTX,
    }
    paired = pairs.get(document_format)
    if paired is None:
        raise CorpusError("admission.paired_source", document_format.value)
    return paired
