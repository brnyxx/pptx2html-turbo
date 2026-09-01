from __future__ import annotations

import re
from typing import NoReturn

from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_ready_manifest_items import (
    QUOTAS,
    ReadyManifestError,
    ReadyManifestFailure,
    build_conformance_items,
)
from evaluate.multiformat_ready_types import (
    ReadyBlind,
    ReadyConformance,
    ReadySecurity,
    ReadySource,
    ReadySourceSet,
    ReadySupport,
)
from evaluate.multiformat_schema import JsonValue

__all__ = [
    "ReadyManifestError",
    "ReadyManifestFailure",
    "build_format_manifest",
]

_SHA256 = re.compile(r"[0-9a-f]{64}").fullmatch
_EXPECTED_COUNTS = {
    ReadyConformance: 100,
    ReadyBlind: 75,
    ReadySecurity: 10,
}


def build_format_manifest(
    contract_digest: str,
    document_format: DocumentFormat,
    sources: ReadySourceSet,
) -> dict[str, JsonValue]:
    if _SHA256(contract_digest) is None:
        _fail(ReadyManifestFailure.DIGEST, document_format, None, "contract")
    selected = [
        source
        for source in sources.sources
        if source.document_format is document_format
    ]
    grouped = _group_sources(document_format, selected)
    supports = [
        support
        for support in sources.supports
        if support.owner_format is document_format
    ]
    _validate_identities(document_format, selected, supports, sources)
    conformance = build_conformance_items(
        document_format, grouped[ReadyConformance], supports
    )
    blind = _build_blind_items(document_format, grouped[ReadyBlind])
    security = _build_security_items(document_format, grouped[ReadySecurity])
    tracks: dict[str, JsonValue] = {
        "conformance": _track_value(100, conformance),
        "blind": _track_value(75, blind),
        "security": _track_value(10, security),
    }
    return {
        "schema_version": 2,
        "status": "READY",
        "format": document_format.value,
        "contract_sha256": contract_digest,
        "stratum_quotas": dict(QUOTAS[document_format]),
        "tracks": tracks,
    }


def _build_blind_items(
    document_format: DocumentFormat, sources: list[ReadySource]
) -> list[dict[str, JsonValue]]:
    result: list[dict[str, JsonValue]] = []
    for source in sorted(sources, key=lambda item: item.source_id):
        details = source.details
        if not isinstance(details, ReadyBlind):
            _fail(
                ReadyManifestFailure.COUNT,
                document_format,
                source.source_id,
                "blind type",
            )
        if details.background != "light":
            _fail(
                ReadyManifestFailure.BACKGROUND,
                document_format,
                source.source_id,
                details.background,
            )
        result.append(
            {
                "id": source.source_id,
                "path": f"sources/blind/{source.source_id}.{document_format.value}",
                "sha256": source.source_sha256,
                "producer": details.producer,
                "source_uri": details.source_uri,
                "template_family": details.template_family,
                "unit_count": source.unit_count,
                "applicable_metrics": list(details.applicable_metrics),
                "background": "#ffffff",
            }
        )
    return result


def _build_security_items(
    document_format: DocumentFormat, sources: list[ReadySource]
) -> list[dict[str, JsonValue]]:
    result: list[dict[str, JsonValue]] = []
    for source in sorted(sources, key=lambda item: item.source_id):
        details = source.details
        if not isinstance(details, ReadySecurity):
            _fail(
                ReadyManifestFailure.COUNT,
                document_format,
                source.source_id,
                "security type",
            )
        result.append(
            {
                "id": source.source_id,
                "path": f"sources/security/{source.source_id}.{document_format.value}",
                "sha256": source.source_sha256,
                "case_family": details.case_family,
                "expected_outcome": details.expected_outcome.value,
            }
        )
    return result


def _track_value(
    expected_count: int, items: list[dict[str, JsonValue]]
) -> dict[str, JsonValue]:
    item_values: list[JsonValue] = list(items)
    return {"expected_count": expected_count, "items": item_values}


def _group_sources(
    document_format: DocumentFormat,
    sources: list[ReadySource],
) -> dict[type[ReadyConformance | ReadyBlind | ReadySecurity], list[ReadySource]]:
    result: dict[
        type[ReadyConformance | ReadyBlind | ReadySecurity], list[ReadySource]
    ] = {ReadyConformance: [], ReadyBlind: [], ReadySecurity: []}
    for source in sources:
        details_type = type(source.details)
        if details_type not in result:
            _fail(ReadyManifestFailure.COUNT, document_format, source.source_id, "type")
        result[details_type].append(source)
    for details_type, expected in _EXPECTED_COUNTS.items():
        if len(result[details_type]) != expected:
            _fail(
                ReadyManifestFailure.COUNT,
                document_format,
                None,
                f"{details_type.__name__}:{len(result[details_type])}",
            )
    return result


def _validate_identities(
    document_format: DocumentFormat,
    selected: list[ReadySource],
    supports: list[ReadySupport],
    source_set: ReadySourceSet,
) -> None:
    ids = [source.source_id for source in selected]
    ids.extend(support.support_id for support in supports)
    if len(ids) != len(set(ids)):
        _fail(ReadyManifestFailure.ID, document_format, None, "duplicate")
    paths = [
        f"sources/{_track(source)}/{source.source_id}.{document_format.value}"
        for source in selected
    ]
    paths.extend(f"sources/support/{support.filename}" for support in supports)
    if len(paths) != len(set(paths)):
        _fail(ReadyManifestFailure.PATH, document_format, None, "duplicate")
    digests = [source.source_sha256 for source in selected]
    digests.extend(support.source_sha256 for support in supports)
    if any(_SHA256(digest) is None for digest in digests):
        _fail(ReadyManifestFailure.DIGEST, document_format, None, "invalid")
    if len(digests) != len(set(digests)):
        _fail(ReadyManifestFailure.DIGEST, document_format, None, "duplicate")
    _validate_support_bindings(document_format, supports, source_set)


def _validate_support_bindings(
    document_format: DocumentFormat,
    supports: list[ReadySupport],
    source_set: ReadySourceSet,
) -> None:
    modern = {
        (source.document_format, source.source_id): source.source_sha256
        for source in source_set.sources
        if isinstance(source.details, ReadyConformance)
    }
    for support in supports:
        selected_digest = modern.get((support.support_format, support.modern_case_id))
        if selected_digest != support.source_sha256:
            _fail(
                ReadyManifestFailure.SUPPORT,
                document_format,
                support.owner_source_id,
                "modern binding",
            )


def _track(source: ReadySource) -> str:
    if isinstance(source.details, ReadyConformance):
        return "conformance"
    if isinstance(source.details, ReadyBlind):
        return "blind"
    return "security"


def _fail(
    failure: ReadyManifestFailure,
    document_format: DocumentFormat,
    source_id: str | None,
    detail: str,
) -> NoReturn:
    raise ReadyManifestError(failure, document_format, source_id, detail)
