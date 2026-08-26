from __future__ import annotations

import hashlib
from dataclasses import replace
from pathlib import Path

from evaluate.multiformat_corpus_contract import corpus_rules
from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_ready_types import (
    ReadyBinaryProvenance,
    ReadyBlind,
    ReadyConformance,
    ReadySecurity,
    ReadySource,
    ReadySourceSet,
    ReadySupport,
)
from evaluate.multiformat_schema import sha256_file
from evaluate.multiformat_security_source import write_security_source
from evaluate.multiformat_source_fixture import write_positive_source
from evaluate.multiformat_strict_json import read_strict_object

ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "evaluate/multiformat/contract.v1.json"
_PAIRS = {
    DocumentFormat.DOC: DocumentFormat.DOCX,
    DocumentFormat.XLS: DocumentFormat.XLSX,
    DocumentFormat.PPT: DocumentFormat.PPTX,
}


def make_manifest_sources(root: Path) -> ReadySourceSet:
    contract = read_strict_object(CONTRACT)
    sources: list[ReadySource] = []
    supports: list[ReadySupport] = []
    modern: dict[tuple[DocumentFormat, int], ReadySource] = {}
    for document_format in DocumentFormat:
        rules = corpus_rules(contract, document_format)
        strata = [name for name, count in rules.quotas.items() for _ in range(count)]
        paired: list[str | None] = [
            name
            for name, count in (rules.paired_quotas or {}).items()
            for _ in range(count)
        ]
        paired.extend([None] * (100 - len(paired)))
        for ordinal, stratum in enumerate(strata, 1):
            source = _conformance_source(
                root, document_format, ordinal, stratum, paired[ordinal - 1]
            )
            sources.append(source)
            modern[document_format, ordinal] = source
        for ordinal in range(75):
            source_id = f"{document_format.value}-blind-{ordinal:03d}"
            sources.append(
                _source(
                    root,
                    document_format,
                    "blind",
                    source_id,
                    ordinal % 3 + 1,
                    ReadyBlind(
                        f"producer-{ordinal % 5}",
                        f"urn:blind:{document_format.value}:{ordinal}",
                        f"template-{document_format.value}-{ordinal}",
                        ("visual", "content", "layout"),
                        "light",
                    ),
                )
            )
        for ordinal, (family, outcome) in enumerate(rules.security_outcomes.items()):
            source_id = f"{document_format.value}-security-{ordinal:03d}"
            sources.append(
                _source(
                    root,
                    document_format,
                    "security",
                    source_id,
                    1,
                    ReadySecurity(family, outcome),
                )
            )
    _add_supports(root, sources, supports, modern)
    return ReadySourceSet(tuple(sources), tuple(supports))


def _conformance_source(
    root: Path,
    document_format: DocumentFormat,
    ordinal: int,
    stratum: str,
    paired_stratum: str | None,
) -> ReadySource:
    source_id = f"{document_format.value}-conformance-{ordinal:03d}"
    provenance = None
    if stratum == "binary-specific":
        provenance = ReadyBinaryProvenance(
            f"producer-{ordinal % 5}",
            f"urn:ready:{document_format.value}:{ordinal}",
            True,
        )
    details = ReadyConformance(
        source_id,
        ordinal,
        stratum,
        paired_stratum if stratum == "paired-legacy" else None,
        hashlib.sha256(source_id.encode()).hexdigest(),
        None,
        provenance,
    )
    return _source(root, document_format, "conformance", source_id, 1, details)


def _source(
    root: Path,
    document_format: DocumentFormat,
    track: str,
    source_id: str,
    count: int,
    details: ReadyConformance | ReadyBlind | ReadySecurity,
) -> ReadySource:
    path = (
        root
        / document_format.value
        / "sources"
        / track
        / f"{source_id}.{document_format.value}"
    )
    path.parent.mkdir(parents=True, exist_ok=True)
    if isinstance(details, ReadySecurity):
        write_security_source(path, document_format, details.case_family)
    else:
        write_positive_source(path, document_format.value, source_id)
    return ReadySource(
        document_format, source_id, path, sha256_file(path), count, details
    )


def _add_supports(
    root: Path,
    sources: list[ReadySource],
    supports: list[ReadySupport],
    modern: dict[tuple[DocumentFormat, int], ReadySource],
) -> None:
    for owner, support_format in _PAIRS.items():
        owned: list[tuple[ReadySource, ReadyConformance]] = []
        for item in sources:
            details = item.details
            if item.document_format is owner and isinstance(details, ReadyConformance):
                owned.append((item, details))
        for source, details in owned[:60]:
            selected = modern[support_format, details.ordinal]
            support_id = f"{owner.value}-support-{selected.source_id}"
            filename = f"{support_id}.{support_format.value}"
            path = root / owner.value / "sources" / "support" / filename
            path.parent.mkdir(parents=True, exist_ok=True)
            _ = path.write_bytes(selected.source_path.read_bytes())
            supports.append(
                ReadySupport(
                    owner,
                    source.source_id,
                    support_format,
                    selected.source_id,
                    support_id,
                    path,
                    selected.source_sha256,
                    filename,
                )
            )
            index = sources.index(source)
            sources[index] = replace(
                source, details=replace(details, support_id=support_id)
            )
