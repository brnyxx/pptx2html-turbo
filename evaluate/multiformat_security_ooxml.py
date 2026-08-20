from __future__ import annotations

import struct
import zipfile
from pathlib import Path, PurePosixPath
from typing import Final

from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_package_validation import (
    MAX_XML_BYTES,
    MAX_ZIP_ENTRIES,
    MAX_ZIP_UNCOMPRESSED_BYTES,
    valid_ooxml_bytes,
)
from evaluate.multiformat_security_ooxml_relationships import (
    has_corrupt_media,
    has_embedded_object,
    has_macro,
    has_relationship_cycle,
    parse_relationships,
    reachable_office_parts,
)

ZIP_LOCAL_HEADER: Final[bytes] = b"PK\x03\x04"
ZIP_END_RECORD: Final[bytes] = b"PK\x05\x06"
ZIP_BOMB_RATIO: Final[int] = 1_000
MAX_RELATIONSHIP_BYTES: Final[int] = 32 * 1024 * 1024
MAX_REACHABLE_BYTES: Final[int] = 64 * 1024 * 1024
OOXML_SPECS: Final[dict[DocumentFormat, tuple[str, str, str]]] = {
    DocumentFormat.DOCX: (
        "word/document.xml",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml",
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}document",
    ),
    DocumentFormat.XLSX: (
        "xl/workbook.xml",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml",
        "{http://schemas.openxmlformats.org/spreadsheetml/2006/main}workbook",
    ),
    DocumentFormat.PPTX: (
        "ppt/presentation.xml",
        "application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml",
        "{http://schemas.openxmlformats.org/presentationml/2006/main}presentation",
    ),
}


def detect_ooxml_security_families(
    path: Path,
    document_format: DocumentFormat,
) -> frozenset[str]:
    try:
        value = path.read_bytes()
    except OSError:
        return frozenset()
    try:
        with zipfile.ZipFile(path) as archive:
            infos = archive.infolist()
            spec = OOXML_SPECS.get(document_format)
            if spec is None or spec[0] not in {info.filename for info in infos}:
                return frozenset()
            return _detect_archive_families(archive, infos)
    except zipfile.BadZipFile:
        spec = OOXML_SPECS.get(document_format)
        repaired = _repair_missing_eocd(value)
        if (
            spec is not None
            and repaired is not None
            and valid_ooxml_bytes(
                repaired,
                *spec,
            )
        ):
            return frozenset({"malformed-zip"})
        return frozenset()


def _detect_archive_families(
    archive: zipfile.ZipFile,
    infos: list[zipfile.ZipInfo],
) -> frozenset[str]:
    families: set[str] = set()
    names = {info.filename for info in infos}
    if any(_unsafe_name(name) for name in names):
        families.add("path-traversal")
    if _is_zip_bomb_inventory(infos):
        families.add("zip-bomb")
        return frozenset(families)
    try:
        relationship_values = _bounded_reads(
            archive,
            [
                info
                for info in infos
                if info.filename.lower().endswith(".rels")
                and info.file_size <= MAX_XML_BYTES
            ],
            MAX_RELATIONSHIP_BYTES,
        )
    except (KeyError, RuntimeError, zipfile.BadZipFile):
        return frozenset()
    if relationship_values is None:
        return frozenset()
    relationships = parse_relationships(relationship_values)
    reachable = reachable_office_parts(names, relationships)
    try:
        reachable_values = _bounded_reads(
            archive,
            [
                info
                for info in infos
                if info.filename in reachable
                and not info.filename.lower().endswith(".rels")
                and info.file_size <= MAX_XML_BYTES
            ],
            MAX_REACHABLE_BYTES,
        )
    except (KeyError, RuntimeError, zipfile.BadZipFile):
        return frozenset()
    if reachable_values is None:
        return frozenset()
    values = {**relationship_values, **reachable_values}
    if any(_is_oversized_xml(info) and info.filename in reachable for info in infos):
        families.add("oversized-xml")
    if any(
        name in reachable
        and name.lower().endswith((".xml", ".rels"))
        and _contains_entity(value)
        for name, value in values.items()
    ):
        families.add("entity-expansion")
    relationship_sources = {*reachable, ""}
    if any(
        external and source in relationship_sources
        for source, _, _, external in relationships
    ):
        families.add("external-relationship")
    if has_macro(values, relationships, relationship_sources):
        families.add("macro-content")
    if has_embedded_object(values, relationships, relationship_sources):
        families.add("embedded-object")
    if has_relationship_cycle(names, relationships, relationship_sources):
        families.add("relationship-cycle")
    if has_corrupt_media(values, relationships, relationship_sources):
        families.add("corrupt-media")
    return frozenset(families)


def _unsafe_name(name: str) -> bool:
    relative = PurePosixPath(name)
    return (
        "\\" in name
        or relative.is_absolute()
        or any(part in {"", ".", ".."} for part in relative.parts)
    )


def _is_zip_bomb(info: zipfile.ZipInfo) -> bool:
    return (
        not info.filename.lower().endswith((".xml", ".rels"))
        and info.file_size > MAX_XML_BYTES
        and info.compress_size > 0
        and info.file_size // info.compress_size >= ZIP_BOMB_RATIO
    )


def _is_zip_bomb_inventory(infos: list[zipfile.ZipInfo]) -> bool:
    return (
        len(infos) > MAX_ZIP_ENTRIES
        or sum(info.file_size for info in infos) > MAX_ZIP_UNCOMPRESSED_BYTES
        or any(_is_zip_bomb(info) for info in infos)
    )


def _bounded_reads(
    archive: zipfile.ZipFile,
    infos: list[zipfile.ZipInfo],
    byte_limit: int,
) -> dict[str, bytes] | None:
    if sum(info.file_size for info in infos) > byte_limit:
        return None
    return {info.filename: archive.read(info) for info in infos}


def _is_oversized_xml(info: zipfile.ZipInfo) -> bool:
    return (
        info.filename.lower().endswith((".xml", ".rels"))
        and info.file_size > MAX_XML_BYTES
    )


def _contains_entity(value: bytes) -> bool:
    upper = value.upper()
    return b"<!DOCTYPE" in upper and b"<!ENTITY" in upper


def _repair_missing_eocd(value: bytes) -> bytes | None:
    if value[:4] != ZIP_LOCAL_HEADER or ZIP_END_RECORD in value[-256:]:
        return None
    central_offset = value.find(b"PK\x01\x02")
    if central_offset <= 0:
        return None
    cursor = central_offset
    entry_count = 0
    while cursor < len(value):
        if value[cursor : cursor + 4] != b"PK\x01\x02" or cursor + 46 > len(value):
            return None
        name_length, extra_length, comment_length = struct.unpack_from(
            "<HHH",
            value,
            cursor + 28,
        )
        cursor += 46 + name_length + extra_length + comment_length
        entry_count += 1
        if entry_count > MAX_ZIP_ENTRIES:
            return None
    if cursor != len(value) or entry_count == 0 or entry_count > 0xFFFF:
        return None
    central_size = len(value) - central_offset
    eocd = struct.pack(
        "<4sHHHHIIH",
        ZIP_END_RECORD,
        0,
        0,
        entry_count,
        entry_count,
        central_size,
        central_offset,
        0,
    )
    return value + eocd
