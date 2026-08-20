from __future__ import annotations

from collections import Counter
from pathlib import Path

from evaluate.multiformat_corpus_items import (
    add_unique,
    canonical_identity,
    canonical_source_uri,
    object_list,
    require_keys,
    track_items,
)
from evaluate.multiformat_corpus_sources import (
    validate_identifier,
    validate_source,
)
from evaluate.multiformat_corpus_types import (
    CorpusError,
    CorpusRules,
    DocumentFormat,
    TrackValidation,
)
from evaluate.multiformat_schema import (
    JsonValue,
    boolean_value,
    integer_value,
    string_list,
    string_value,
)


def validate_conformance(
    track: dict[str, JsonValue],
    root: Path,
    document_format: DocumentFormat,
    rules: CorpusRules,
) -> TrackValidation:
    items = track_items(track, rules.conformance_units, "conformance")
    item_ids: set[str] = set()
    source_paths: set[str] = set()
    source_hashes: set[str] = set()
    unit_ids: set[str] = set()
    strata: Counter[str] = Counter()
    paired_strata: Counter[str] = Counter()
    for item in items:
        require_keys(
            item,
            {"id", "path", "sha256", "units", "paired_source", "provenance"},
            "conformance.item",
        )
        source = validate_source(
            item,
            root,
            document_format,
            require_valid_format=True,
        )
        add_unique(item_ids, source.item_id, "conformance.id")
        add_unique(source_paths, source.relative_path, "conformance.path")
        add_unique(source_hashes, source.digest, "conformance.sha256")
        units = object_list(item, "units", "conformance.units")
        if not units:
            raise CorpusError("conformance.units", source.item_id)
        primary_values: set[str] = set()
        item_paired_strata: Counter[str] = Counter()
        for ordinal, unit in enumerate(units, start=1):
            require_keys(
                unit,
                {
                    "id",
                    "ordinal",
                    "primary_stratum",
                    "paired_stratum",
                    "secondary_features",
                },
                "conformance.unit",
            )
            unit_id = string_value(unit, "id")
            validate_identifier(unit_id, "conformance.unit.id")
            add_unique(unit_ids, unit_id, "conformance.unit.id")
            if integer_value(unit, "ordinal") != ordinal:
                raise CorpusError("conformance.unit.ordinal", unit_id)
            primary = string_value(unit, "primary_stratum")
            if primary not in rules.quotas:
                raise CorpusError("conformance.stratum", primary)
            primary_values.add(primary)
            paired_stratum = _optional_string(unit, "paired_stratum")
            if paired_stratum is not None:
                paired_strata[paired_stratum] += 1
                item_paired_strata[paired_stratum] += 1
            secondary = string_list(unit, "secondary_features")
            if len(secondary) != len(set(secondary)):
                raise CorpusError("conformance.secondary_features", unit_id)
            strata[primary] += 1
        _validate_item_provenance(
            item,
            root,
            rules,
            primary_values,
            item_paired_strata,
            len(units),
            item_ids,
            source_paths,
            source_hashes,
        )
    if len(unit_ids) != rules.conformance_units:
        raise CorpusError("conformance.count", str(len(unit_ids)))
    if dict(strata) != rules.quotas:
        raise CorpusError("conformance.stratum", repr(dict(strata)))
    if rules.paired_quotas is not None and dict(paired_strata) != rules.paired_quotas:
        raise CorpusError("conformance.paired_stratum", repr(dict(paired_strata)))
    if rules.paired_quotas is None and paired_strata:
        raise CorpusError("conformance.paired_stratum", "not applicable")
    return TrackValidation(
        len(unit_ids),
        frozenset(item_ids),
        frozenset(source_paths),
        frozenset(source_hashes),
    )


def _validate_item_provenance(
    item: dict[str, JsonValue],
    root: Path,
    rules: CorpusRules,
    primary_values: set[str],
    item_paired_strata: Counter[str],
    unit_count: int,
    item_ids: set[str],
    source_paths: set[str],
    source_hashes: set[str],
) -> None:
    paired_source = _optional_object(item, "paired_source")
    provenance = _optional_object(item, "provenance")
    if rules.paired_format is None:
        if paired_source is not None or provenance is not None or item_paired_strata:
            raise CorpusError("conformance.provenance", "not applicable")
        return
    if len(primary_values) != 1:
        raise CorpusError("conformance.stratum", "legacy source mixes strata")
    primary = next(iter(primary_values))
    if primary == "paired-legacy":
        if (
            paired_source is None
            or provenance is not None
            or sum(item_paired_strata.values()) != unit_count
        ):
            raise CorpusError("conformance.paired_source", "required")
        require_keys(paired_source, {"id", "path", "sha256"}, "paired_source")
        source = validate_source(
            paired_source,
            root,
            rules.paired_format,
            require_valid_format=True,
        )
        add_unique(item_ids, source.item_id, "conformance.id")
        add_unique(source_paths, source.relative_path, "conformance.path")
        add_unique(source_hashes, source.digest, "conformance.sha256")
        return
    if (
        primary != "binary-specific"
        or paired_source is not None
        or provenance is None
        or item_paired_strata
    ):
        raise CorpusError("conformance.provenance", "invalid legacy source")
    require_keys(
        provenance,
        {"producer", "source_uri", "independently_authored"},
        "conformance.provenance",
    )
    canonical_identity(
        string_value(provenance, "producer"),
        "conformance.producer",
    )
    canonical_source_uri(string_value(provenance, "source_uri"))
    if not boolean_value(provenance, "independently_authored"):
        raise CorpusError("conformance.provenance", "independence required")


def _optional_object(
    values: dict[str, JsonValue],
    field: str,
) -> dict[str, JsonValue] | None:
    value = values.get(field)
    if value is None:
        return None
    if not isinstance(value, dict):
        raise CorpusError(f"conformance.{field}", "must be an object or null")
    return value


def _optional_string(values: dict[str, JsonValue], field: str) -> str | None:
    value = values.get(field)
    if value is None:
        return None
    if not isinstance(value, str) or not value:
        raise CorpusError(f"conformance.{field}", "must be a string or null")
    return value
