from __future__ import annotations

from pathlib import Path

from evaluate.multiformat_corpus_items import (
    add_unique,
    canonical_identity,
    canonical_source_uri,
    require_keys,
    track_items,
    validate_background,
)
from evaluate.multiformat_corpus_sources import (
    validate_source,
)
from evaluate.multiformat_corpus_types import (
    CorpusError,
    DocumentFormat,
    SecurityOutcome,
    TrackValidation,
)
from evaluate.multiformat_schema import (
    JsonValue,
    integer_value,
    object_value,
    string_list,
    string_value,
)


def validate_incomplete_tracks(
    tracks: dict[str, JsonValue],
    expected_counts: dict[str, int],
) -> None:
    require_keys(tracks, {"conformance", "blind", "security"}, "tracks")
    for name, expected_count in expected_counts.items():
        track = object_value(tracks, name)
        items = track_items(track, expected_count, name)
        if items:
            raise CorpusError(f"{name}.items", "INCOMPLETE manifest must be empty")


def validate_blind(
    track: dict[str, JsonValue],
    root: Path,
    document_format: DocumentFormat,
    expected_count: int,
) -> tuple[TrackValidation, int]:
    items = track_items(track, expected_count, "blind")
    item_ids: set[str] = set()
    source_paths: set[str] = set()
    source_hashes: set[str] = set()
    producers: set[str] = set()
    templates: set[str] = set()
    source_uris: set[str] = set()
    for item in items:
        require_keys(
            item,
            {
                "id",
                "path",
                "sha256",
                "producer",
                "source_uri",
                "template_family",
                "unit_count",
                "applicable_metrics",
                "background",
            },
            "blind.item",
        )
        source = validate_source(
            item,
            root,
            document_format,
            require_valid_format=True,
        )
        add_unique(item_ids, source.item_id, "blind.id")
        add_unique(source_paths, source.relative_path, "blind.path")
        add_unique(source_hashes, source.digest, "blind.sha256")
        producer = canonical_identity(
            string_value(item, "producer"),
            "blind.producer",
        )
        producers.add(producer)
        template = canonical_identity(
            string_value(item, "template_family"),
            "blind.template_family",
        )
        add_unique(templates, template, "blind.template_family")
        source_uri = canonical_source_uri(string_value(item, "source_uri"))
        add_unique(source_uris, source_uri, "blind.source_uri")
        metrics = string_list(item, "applicable_metrics")
        if (
            "visual" not in metrics
            or len(metrics) != len(set(metrics))
            or not set(metrics).issubset({"visual", "content", "layout"})
        ):
            raise CorpusError("blind.applicable_metrics", source.item_id)
        validate_background(string_value(item, "background"), "blind.background")
        if integer_value(item, "unit_count") <= 0:
            raise CorpusError("blind.unit_count", source.item_id)
    if len(items) != expected_count:
        raise CorpusError("blind.count", str(len(items)))
    if len(producers) < 5:
        raise CorpusError("blind.producers", str(len(producers)))
    return (
        TrackValidation(
            len(items),
            frozenset(item_ids),
            frozenset(source_paths),
            frozenset(source_hashes),
        ),
        len(producers),
    )


def validate_security(
    track: dict[str, JsonValue],
    root: Path,
    document_format: DocumentFormat,
    expected_count: int,
    expected_outcomes: dict[str, SecurityOutcome],
) -> TrackValidation:
    items = track_items(track, expected_count, "security")
    item_ids: set[str] = set()
    source_paths: set[str] = set()
    source_hashes: set[str] = set()
    families: set[str] = set()
    for item in items:
        require_keys(
            item,
            {"id", "path", "sha256", "case_family", "expected_outcome"},
            "security.item",
        )
        family = string_value(item, "case_family")
        add_unique(families, family, "security.case_family")
        try:
            outcome = SecurityOutcome(string_value(item, "expected_outcome"))
        except ValueError as error:
            raise CorpusError("security.expected_outcome", family) from error
        if expected_outcomes.get(family) is not outcome:
            raise CorpusError("security.expected_outcome", family)
        source = validate_source(
            item,
            root,
            document_format,
            require_valid_format=outcome is SecurityOutcome.SAFE_CONVERT,
        )
        add_unique(item_ids, source.item_id, "security.id")
        add_unique(source_paths, source.relative_path, "security.path")
        add_unique(source_hashes, source.digest, "security.sha256")
    if len(items) != expected_count:
        raise CorpusError("security.count", str(len(items)))
    if families != set(expected_outcomes):
        raise CorpusError("security.case_family", "contract mismatch")
    return TrackValidation(
        len(items),
        frozenset(item_ids),
        frozenset(source_paths),
        frozenset(source_hashes),
    )
