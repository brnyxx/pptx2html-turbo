from __future__ import annotations

from collections import Counter
from dataclasses import dataclass
from enum import StrEnum
from typing import NoReturn

from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_ready_types import (
    ReadyConformance,
    ReadySource,
    ReadySupport,
)
from evaluate.multiformat_schema import JsonValue

FULL_METRICS: list[JsonValue] = ["visual", "content", "layout"]


def _quota(names: str, counts: tuple[int, ...]) -> dict[str, int]:
    return dict(zip(names.split(), counts, strict=True))


QUOTAS: dict[DocumentFormat, dict[str, int]] = {
    DocumentFormat.PPTX: _quota(
        "text shapes-connectors images-effects tables-charts masters-layouts-groups international fallback-edge",
        (20, 20, 15, 15, 15, 10, 5),
    ),
    DocumentFormat.DOCX: _quota(
        "text-typography sections-headers-footers tables-images-shapes lists-fields-references international mixed-stress",
        (25, 20, 20, 15, 10, 10),
    ),
    DocumentFormat.DOC: _quota("paired-legacy binary-specific", (60, 40)),
    DocumentFormat.XLSX: _quota(
        "values-formulas styles-conditional-formats print-layout charts-images-shapes international-formats mixed-stress",
        (25, 20, 20, 15, 10, 10),
    ),
    DocumentFormat.XLS: _quota("paired-legacy binary-specific", (60, 40)),
    DocumentFormat.PPT: _quota("paired-legacy binary-specific", (60, 40)),
    DocumentFormat.PDF: _quota(
        "text-fonts vector-transparency raster-color-space page-geometry forms-annotations-links international mixed-edge",
        (20, 20, 15, 15, 10, 10, 10),
    ),
}
PAIRS = {
    DocumentFormat.DOC: DocumentFormat.DOCX,
    DocumentFormat.XLS: DocumentFormat.XLSX,
    DocumentFormat.PPT: DocumentFormat.PPTX,
}
PAIRED_QUOTAS: dict[DocumentFormat, dict[str, int]] = {
    DocumentFormat.DOC: dict(
        zip(QUOTAS[DocumentFormat.DOCX], (15, 12, 12, 9, 6, 6), strict=True)
    ),
    DocumentFormat.XLS: dict(
        zip(QUOTAS[DocumentFormat.XLSX], (15, 12, 12, 9, 6, 6), strict=True)
    ),
    DocumentFormat.PPT: dict(
        zip(QUOTAS[DocumentFormat.PPTX], (12, 12, 9, 9, 9, 6, 3), strict=True)
    ),
}


class ReadyManifestFailure(StrEnum):
    COUNT = "count"
    ID = "id"
    PATH = "path"
    DIGEST = "digest"
    STRATUM = "stratum"
    BACKGROUND = "background"
    SUPPORT = "support"
    CONFORMANCE = "conformance"


@dataclass(frozen=True, slots=True)
class ReadyManifestError(Exception):
    failure: ReadyManifestFailure
    document_format: DocumentFormat
    source_id: str | None
    detail: str


def build_conformance_items(
    document_format: DocumentFormat,
    sources: list[ReadySource],
    supports: list[ReadySupport],
) -> list[dict[str, JsonValue]]:
    ordered = sorted(sources, key=_conformance_order)
    strata = Counter[str]()
    paired_strata = Counter[str]()
    support_map = {(item.owner_source_id, item.support_id): item for item in supports}
    if len(support_map) != len(supports):
        _fail(ReadyManifestFailure.SUPPORT, document_format, None, "duplicate binding")
    items: list[dict[str, JsonValue]] = []
    for expected_ordinal, source in enumerate(ordered, 1):
        details = _conformance(source, document_format)
        if details.ordinal != expected_ordinal:
            _fail_source(
                ReadyManifestFailure.CONFORMANCE, document_format, source, "ordinal"
            )
        strata[details.primary_stratum] += 1
        if details.paired_stratum is not None:
            paired_strata[details.paired_stratum] += 1
        paired_source = _paired_source(document_format, source, details, support_map)
        provenance = _provenance(document_format, source, details)
        secondary_features: list[JsonValue] = [details.feature_seed]
        unit: dict[str, JsonValue] = {
            "id": source.source_id,
            "ordinal": 1,
            "primary_stratum": details.primary_stratum,
            "paired_stratum": details.paired_stratum,
            "applicable_metrics": FULL_METRICS,
            "background": "#ffffff",
            "secondary_features": secondary_features,
        }
        items.append(
            {
                "id": source.source_id,
                "path": _path("conformance", source, document_format),
                "sha256": source.source_sha256,
                "paired_source": paired_source,
                "provenance": provenance,
                "units": [unit],
            }
        )
    if dict(strata) != QUOTAS[document_format]:
        _fail(ReadyManifestFailure.STRATUM, document_format, None, repr(dict(strata)))
    expected_paired = PAIRED_QUOTAS.get(document_format, {})
    if dict(paired_strata) != expected_paired:
        _fail(
            ReadyManifestFailure.STRATUM,
            document_format,
            None,
            repr(dict(paired_strata)),
        )
    if support_map:
        _fail(ReadyManifestFailure.SUPPORT, document_format, None, "unused binding")
    return items


def _paired_source(
    document_format: DocumentFormat,
    source: ReadySource,
    details: ReadyConformance,
    support_map: dict[tuple[str, str], ReadySupport],
) -> dict[str, JsonValue] | None:
    if details.primary_stratum != "paired-legacy":
        if details.support_id is not None or details.paired_stratum is not None:
            _fail_source(
                ReadyManifestFailure.SUPPORT, document_format, source, "not paired"
            )
        return None
    if details.support_id is None or details.paired_stratum is None:
        _fail_source(ReadyManifestFailure.SUPPORT, document_format, source, "missing")
    support = support_map.pop((source.source_id, details.support_id), None)
    expected_format = PAIRS.get(document_format)
    expected_id = (
        f"{document_format.value}-support-{support.modern_case_id}" if support else ""
    )
    if support is None or (
        support.owner_format is not document_format
        or support.support_format is not expected_format
        or support.support_id != expected_id
        or support.filename != f"{expected_id}.{support.support_format.value}"
    ):
        _fail_source(ReadyManifestFailure.SUPPORT, document_format, source, "identity")
    return {
        "id": support.support_id,
        "path": f"sources/support/{support.filename}",
        "sha256": support.source_sha256,
    }


def _provenance(
    document_format: DocumentFormat,
    source: ReadySource,
    details: ReadyConformance,
) -> dict[str, JsonValue] | None:
    provenance = details.provenance
    if details.primary_stratum != "binary-specific":
        if provenance is not None:
            _fail_source(
                ReadyManifestFailure.CONFORMANCE, document_format, source, "provenance"
            )
        return None
    if provenance is None:
        _fail_source(
            ReadyManifestFailure.CONFORMANCE, document_format, source, "provenance"
        )
    return {
        "producer": provenance.producer,
        "source_uri": provenance.source_uri,
        "independently_authored": provenance.independently_authored,
    }


def _conformance(
    source: ReadySource, document_format: DocumentFormat
) -> ReadyConformance:
    details = source.details
    if not isinstance(details, ReadyConformance) or source.unit_count != 1:
        _fail_source(ReadyManifestFailure.CONFORMANCE, document_format, source, "type")
    if details.primary_stratum not in QUOTAS[document_format]:
        _fail_source(
            ReadyManifestFailure.STRATUM,
            document_format,
            source,
            details.primary_stratum,
        )
    return details


def _conformance_order(source: ReadySource) -> int:
    details = source.details
    return details.ordinal if isinstance(details, ReadyConformance) else -1


def _path(track: str, source: ReadySource, document_format: DocumentFormat) -> str:
    return f"sources/{track}/{source.source_id}.{document_format.value}"


def _fail_source(
    failure: ReadyManifestFailure,
    document_format: DocumentFormat,
    source: ReadySource,
    detail: str,
) -> NoReturn:
    _fail(failure, document_format, source.source_id, detail)


def _fail(
    failure: ReadyManifestFailure,
    document_format: DocumentFormat,
    source_id: str | None,
    detail: str,
) -> NoReturn:
    raise ReadyManifestError(failure, document_format, source_id, detail)
