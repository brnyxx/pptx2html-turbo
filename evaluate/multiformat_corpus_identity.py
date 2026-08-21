from __future__ import annotations

import hashlib
import json
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Final

from evaluate.jcs import JcsError, canonicalize
from evaluate.multiformat_contract import contract_digest
from evaluate.multiformat_corpus_items import object_list, require_keys
from evaluate.multiformat_corpus_sources import resolve_source_path, validate_identifier
from evaluate.multiformat_corpus_types import CorpusError, CorpusStatus, DocumentFormat
from evaluate.multiformat_schema import (
    JsonValue,
    integer_value,
    object_value,
    sha256_file,
    sha256_value,
    string_list,
    string_value,
)
from evaluate.multiformat_strict_json import read_strict_object

_REQUIRED_COUNTS: Final = {"conformance": 100, "blind": 75, "security": 10}
_ALLOWED_TRACKS: Final = frozenset({*_REQUIRED_COUNTS, "support"})
_EVIDENCE_KINDS: Final = frozenset({"extraction", "fonts", "render"})
_MANIFEST_FIELDS: Final = {
    "schema_version",
    "status",
    "corpus_revision",
    "contract_sha256",
    "aggregate_sha256",
    "per_format_counts",
    "stratum_quotas",
    "sources",
    "admitted_at",
    "project_revision",
}


@dataclass(frozen=True, slots=True)
class AdmittedCorpusValidation:
    status: CorpusStatus
    aggregate_sha256: str | None
    source_count: int


def admitted_corpus_digest(manifest: dict[str, JsonValue]) -> str:
    """Hash a manifest with source records ordered by their semantic identity."""
    payload = {
        key: value for key, value in manifest.items() if key != "aggregate_sha256"
    }
    records = object_list(manifest, "sources", "manifest.sources")
    payload["sources"] = sorted(records, key=_record_sort_key)
    try:
        return hashlib.sha256(canonicalize(payload)).hexdigest()
    except JcsError as error:
        raise CorpusError("manifest.aggregate_sha256", str(error)) from error


def validate_admitted_corpus(
    contract_path: Path,
    corpus_root: Path,
) -> AdmittedCorpusValidation:
    """Validate a READY aggregate corpus, or return INCOMPLETE before READY."""
    if not (corpus_root / "READY").is_file():
        return AdmittedCorpusValidation(CorpusStatus.INCOMPLETE, None, 0)
    try:
        return _validate_ready_corpus(contract_path, corpus_root)
    except CorpusError:
        raise
    except (
        OSError,
        UnicodeError,
        json.JSONDecodeError,
        TypeError,
        ValueError,
    ) as error:
        raise CorpusError("manifest.schema", str(error)) from error


def _validate_ready_corpus(
    contract_path: Path,
    corpus_root: Path,
) -> AdmittedCorpusValidation:
    manifest = read_strict_object(corpus_root / "manifest.json")
    contract = read_strict_object(contract_path)
    require_keys(manifest, _MANIFEST_FIELDS, "manifest.schema")
    if integer_value(manifest, "schema_version") != 2:
        raise CorpusError("manifest.schema_version", "expected 2")
    if string_value(manifest, "status") != CorpusStatus.READY.value:
        raise CorpusError("manifest.status", "READY marker requires READY manifest")
    string_value(manifest, "corpus_revision")
    _validate_revision(string_value(manifest, "project_revision"))
    _validate_timestamp(string_value(manifest, "admitted_at"))
    if sha256_value(manifest, "contract_sha256") != contract_digest(contract_path):
        raise CorpusError("manifest.contract_sha256", "contract mismatch")

    required_formats = string_list(contract, "required_formats")
    if required_formats != [
        document_format.value for document_format in DocumentFormat
    ]:
        raise CorpusError("contract.required_formats", "expected all seven formats")
    _validate_counts(manifest, required_formats)
    if object_value(manifest, "stratum_quotas") != object_value(
        contract, "stratum_quotas"
    ):
        raise CorpusError("manifest.stratum_quotas", "contract mismatch")
    records = object_list(manifest, "sources", "manifest.sources")
    _validate_records(records, corpus_root, frozenset(required_formats))
    expected_digest = admitted_corpus_digest(manifest)
    if sha256_value(manifest, "aggregate_sha256") != expected_digest:
        raise CorpusError("manifest.aggregate_sha256", "identity mismatch")
    return AdmittedCorpusValidation(CorpusStatus.READY, expected_digest, len(records))


def _validate_counts(manifest: dict[str, JsonValue], formats: list[str]) -> None:
    counts = object_value(manifest, "per_format_counts")
    if set(counts) != set(formats):
        raise CorpusError("manifest.per_format_counts", "format mismatch")
    for document_format in formats:
        values = object_value(counts, document_format)
        require_keys(values, set(_REQUIRED_COUNTS), "manifest.per_format_counts")
        actual = {track: integer_value(values, track) for track in _REQUIRED_COUNTS}
        if actual != _REQUIRED_COUNTS:
            raise CorpusError("manifest.per_format_counts", document_format)


def _validate_records(
    records: list[dict[str, JsonValue]],
    root: Path,
    formats: frozenset[str],
) -> None:
    identities: set[tuple[str, str]] = set()
    totals = {
        (document_format, track): 0
        for document_format in formats
        for track in _REQUIRED_COUNTS
    }
    paths: set[str] = set()
    for record in records:
        require_keys(
            record,
            {"format", "id", "track", "path", "sha256", "unit_count", "evidence"},
            "source.record",
        )
        document_format = string_value(record, "format")
        item_id = string_value(record, "id")
        track = string_value(record, "track")
        if document_format not in formats or track not in _ALLOWED_TRACKS:
            raise CorpusError("source.record", f"{document_format}:{track}")
        validate_identifier(item_id, "source.id")
        identity = (document_format, item_id)
        if identity in identities:
            raise CorpusError("source.id", f"{document_format}:{item_id}")
        identities.add(identity)
        relative_path = string_value(record, "path")
        if relative_path in paths:
            raise CorpusError("source.path", relative_path)
        paths.add(relative_path)
        source = resolve_source_path(root, relative_path)
        if source.suffix.lower() != f".{document_format}":
            raise CorpusError("source.format", relative_path)
        if sha256_file(source) != sha256_value(record, "sha256"):
            raise CorpusError("source.sha256", relative_path)
        unit_count = integer_value(record, "unit_count")
        if track == "support":
            if unit_count != 0:
                raise CorpusError("source.unit_count", item_id)
        else:
            if unit_count <= 0:
                raise CorpusError("source.unit_count", item_id)
            totals[(document_format, track)] += unit_count
        _validate_evidence(object_value(record, "evidence"), root)
    expected = {
        (document_format, track): count
        for document_format in formats
        for track, count in _REQUIRED_COUNTS.items()
    }
    if totals != expected:
        raise CorpusError("source.count", "record totals do not match contract")


def _validate_evidence(values: dict[str, JsonValue], root: Path) -> None:
    require_keys(values, set(_EVIDENCE_KINDS), "source.evidence")
    for kind in _EVIDENCE_KINDS:
        binding = object_value(values, kind)
        require_keys(binding, {"path", "sha256"}, "source.evidence")
        relative_path = string_value(binding, "path")
        try:
            path = resolve_source_path(root, relative_path)
        except CorpusError as error:
            raise CorpusError("evidence.path", relative_path) from error
        if sha256_file(path) != sha256_value(binding, "sha256"):
            raise CorpusError("evidence.sha256", path.as_posix())


def _record_sort_key(record: dict[str, JsonValue]) -> tuple[str, str, str, str]:
    return (
        string_value(record, "format"),
        string_value(record, "id"),
        string_value(record, "track"),
        string_value(record, "path"),
    )


def _validate_revision(value: str) -> None:
    if len(value) != 40 or any(
        character not in "0123456789abcdef" for character in value
    ):
        raise CorpusError("manifest.project_revision", value)


def _validate_timestamp(value: str) -> None:
    if not value.endswith("Z"):
        raise CorpusError("manifest.admitted_at", value)
    try:
        timestamp = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError as error:
        raise CorpusError("manifest.admitted_at", value) from error
    if timestamp.tzinfo != timezone.utc:
        raise CorpusError("manifest.admitted_at", value)
