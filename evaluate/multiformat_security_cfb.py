from __future__ import annotations

import struct
from pathlib import Path
from typing import Final

from evaluate.multiformat_cfb import (
    CFBF_MAGIC,
    END_OF_CHAIN,
)
from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_package_validation import MAX_SOURCE_BYTES
from evaluate.multiformat_security_cfb_parser import (
    has_difat_overflow,
    parse_cfb_structure,
    plausible_cfb_header,
)
from evaluate.multiformat_security_cfb_streams import (
    has_directory_cycle,
    has_embedded_storage,
    has_external_link,
    has_mini_stream_corruption,
    has_regular_fat_cycle,
    has_truncated_stream,
    root_sibling_indices,
    root_tree_indices,
    storage_has_descendant,
)
from evaluate.multiformat_security_cfb_types import CfbStructure

PRIMARY_STREAMS: Final[dict[DocumentFormat, frozenset[str]]] = {
    DocumentFormat.DOC: frozenset({"worddocument"}),
    DocumentFormat.XLS: frozenset({"workbook", "book"}),
    DocumentFormat.PPT: frozenset({"powerpoint document"}),
}


def is_coherent_cfb(value: bytes) -> bool:
    if value[:8] != CFBF_MAGIC or has_difat_overflow(value):
        return False
    structure, directory_fat_cycle = parse_cfb_structure(value)
    return structure is not None and not directory_fat_cycle


def has_cfb_macro_storage(value: bytes) -> bool:
    if value[:8] != CFBF_MAGIC or has_difat_overflow(value):
        return False
    structure, directory_fat_cycle = parse_cfb_structure(value)
    return (
        structure is not None
        and not directory_fat_cycle
        and storage_has_descendant(
            structure.entries,
            "vba",
            "_vba_project",
        )
    )


def detect_cfb_security_families(
    path: Path,
    document_format: DocumentFormat,
) -> frozenset[str]:
    try:
        value = path.read_bytes()
    except OSError:
        return frozenset()
    if value[:8] != CFBF_MAGIC:
        return (
            frozenset({"malformed-cfbf"})
            if plausible_cfb_header(value)
            and _coherent_after_header_repair(
                value,
                document_format,
                repair_difat=False,
            )
            else frozenset()
        )
    if has_difat_overflow(value):
        return (
            frozenset({"difat-overflow"})
            if _coherent_after_header_repair(
                value,
                document_format,
                repair_difat=True,
            )
            else frozenset()
        )
    structure, directory_fat_cycle = parse_cfb_structure(value)
    if directory_fat_cycle:
        return frozenset({"fat-cycle"})
    if structure is None:
        return frozenset()
    if not _has_expected_primary_stream(structure, document_format):
        return frozenset()
    return _detect_structure_families(value, structure)


def _detect_structure_families(
    value: bytes,
    structure: CfbStructure,
) -> frozenset[str]:
    families: set[str] = set()
    if has_regular_fat_cycle(structure):
        families.add("fat-cycle")
    if has_directory_cycle(structure.entries):
        families.add("directory-cycle")
    if has_mini_stream_corruption(value, structure):
        families.add("mini-stream-corruption")
    if has_truncated_stream(structure):
        families.add("truncated-stream")
    if any(
        structure.entries[index].stream_size > MAX_SOURCE_BYTES
        for index in root_tree_indices(structure.entries)
    ):
        families.add("oversized-stream")
    if has_external_link(value, structure):
        families.add("external-link")
    if storage_has_descendant(structure.entries, "vba", "_vba_project"):
        families.add("macro-storage")
    if has_embedded_storage(structure.entries):
        families.add("embedded-object")
    return frozenset(families)


def _coherent_after_header_repair(
    value: bytes,
    document_format: DocumentFormat,
    *,
    repair_difat: bool,
) -> bool:
    repaired = bytearray(value)
    repaired[:8] = CFBF_MAGIC
    if repair_difat:
        struct.pack_into("<II", repaired, 68, END_OF_CHAIN, 0)
    structure, directory_cycle = parse_cfb_structure(bytes(repaired))
    return (
        structure is not None
        and not directory_cycle
        and _has_expected_primary_stream(structure, document_format)
    )


def _has_expected_primary_stream(
    structure: CfbStructure,
    document_format: DocumentFormat,
) -> bool:
    expected = PRIMARY_STREAMS.get(document_format)
    if expected is None or not structure.entries:
        return False
    for index in root_sibling_indices(structure.entries):
        entry = structure.entries[index]
        if entry.object_type == 2 and entry.name.casefold() in expected:
            return True
    return False
