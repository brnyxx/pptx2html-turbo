from __future__ import annotations

import zipfile
from pathlib import Path, PurePosixPath
from typing import Final
from xml.etree import ElementTree

MAX_SOURCE_BYTES: Final[int] = 100 * 1024 * 1024
MAX_ZIP_ENTRIES: Final[int] = 10_000
MAX_ZIP_UNCOMPRESSED_BYTES: Final[int] = 500 * 1024 * 1024
MAX_ZIP_ENTRY_BYTES: Final[int] = 100 * 1024 * 1024
MAX_ZIP_NAME_LENGTH: Final[int] = 512
MAX_XML_BYTES: Final[int] = 16 * 1024 * 1024
CONTENT_TYPES_NS: Final[str] = (
    "http://schemas.openxmlformats.org/package/2006/content-types"
)
RELATIONSHIPS_NS: Final[str] = (
    "http://schemas.openxmlformats.org/package/2006/relationships"
)
OFFICE_RELATIONSHIP_TYPES: Final[frozenset[str]] = frozenset(
    {
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument",
        "http://purl.oclc.org/ooxml/officeDocument/relationships/officeDocument",
    }
)
OFFICE_MAIN_CONTENT_TYPES: Final[frozenset[str]] = frozenset(
    {
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml",
        "application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml",
    }
)


class PackageValidationError(Exception):
    pass


def valid_ooxml(
    path: Path,
    main_part: str,
    main_content_type: str,
    main_root: str,
) -> bool:
    if not _bounded_file(path):
        return False
    try:
        with zipfile.ZipFile(path) as archive:
            infos = archive.infolist()
            if not _valid_zip_inventory(infos):
                return False
            names = {info.filename for info in infos}
            required = {"[Content_Types].xml", "_rels/.rels", main_part}
            if not required.issubset(names):
                return False
            content_types = _bounded_zip_read(archive, "[Content_Types].xml")
            relationships = _bounded_zip_read(archive, "_rels/.rels")
            main_xml = _bounded_zip_read(archive, main_part)
    except (OSError, KeyError, zipfile.BadZipFile, PackageValidationError):
        return False
    return (
        _content_type_matches(content_types, main_part, main_content_type)
        and _root_relationship_matches(relationships, main_part)
        and _xml_root_matches(main_xml, main_root)
    )


def bounded_source(path: Path) -> bool:
    return _bounded_file(path)


def _bounded_file(path: Path) -> bool:
    try:
        size = path.stat().st_size
    except OSError:
        return False
    return 0 < size <= MAX_SOURCE_BYTES


def _valid_zip_inventory(infos: list[zipfile.ZipInfo]) -> bool:
    if not 0 < len(infos) <= MAX_ZIP_ENTRIES:
        return False
    names: set[str] = set()
    total_uncompressed = 0
    for info in infos:
        name = info.filename
        relative = PurePosixPath(name)
        if (
            len(name) > MAX_ZIP_NAME_LENGTH
            or "\\" in name
            or relative.is_absolute()
            or any(part in {"", ".", ".."} for part in relative.parts)
            or name in names
            or info.flag_bits & 1
            or info.file_size > MAX_ZIP_ENTRY_BYTES
        ):
            return False
        names.add(name)
        total_uncompressed += info.file_size
        if total_uncompressed > MAX_ZIP_UNCOMPRESSED_BYTES:
            return False
    return True


def _bounded_zip_read(archive: zipfile.ZipFile, name: str) -> bytes:
    info = archive.getinfo(name)
    if info.file_size > MAX_XML_BYTES:
        raise PackageValidationError("XML part exceeds the bounded size")
    value = archive.read(info)
    if b"<!DOCTYPE" in value.upper() or b"<!ENTITY" in value.upper():
        raise PackageValidationError("DTD content is forbidden")
    return value


def _content_type_matches(value: bytes, main_part: str, expected: str) -> bool:
    root = _parse_xml(value)
    if root is None or root.tag != f"{{{CONTENT_TYPES_NS}}}Types":
        return False
    part_name = f"/{main_part}"
    main_overrides = [
        (
            child.attrib.get("PartName"),
            child.attrib.get("ContentType"),
        )
        for child in root
        if child.tag == f"{{{CONTENT_TYPES_NS}}}Override"
        and child.attrib.get("ContentType") in OFFICE_MAIN_CONTENT_TYPES
    ]
    return main_overrides == [(part_name, expected)]


def _root_relationship_matches(value: bytes, main_part: str) -> bool:
    root = _parse_xml(value)
    if root is None or root.tag != f"{{{RELATIONSHIPS_NS}}}Relationships":
        return False
    office_relationships = [
        child
        for child in root
        if child.tag == f"{{{RELATIONSHIPS_NS}}}Relationship"
        and child.attrib.get("Type") in OFFICE_RELATIONSHIP_TYPES
    ]
    return len(office_relationships) == 1 and (
        office_relationships[0].attrib.get("TargetMode", "Internal") == "Internal"
        and office_relationships[0].attrib.get("Target", "").lstrip("/") == main_part
    )


def _xml_root_matches(value: bytes, expected: str) -> bool:
    root = _parse_xml(value)
    return root is not None and root.tag == expected


def _parse_xml(value: bytes) -> ElementTree.Element | None:
    try:
        return ElementTree.fromstring(value)
    except ElementTree.ParseError:
        return None
