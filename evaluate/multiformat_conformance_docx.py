from __future__ import annotations

import io
import zipfile
from typing import Final
from xml.etree import ElementTree

from evaluate.multiformat_conformance_docx_parts import case_parts
from evaluate.multiformat_package_validation import valid_ooxml_bytes
from evaluate.multiformat_schema import JsonValue

ENTRY_TIMESTAMP: Final[tuple[int, int, int, int, int, int]] = (1980, 1, 1, 0, 0, 0)
ENTRY_MODE: Final[int] = 0o100644 << 16
EXPECTED_PARTS: Final[tuple[str, ...]] = (
    "[Content_Types].xml",
    "_rels/.rels",
    "word/document.xml",
    "word/styles.xml",
    "word/numbering.xml",
    "word/header1.xml",
    "word/footer1.xml",
    "word/_rels/document.xml.rels",
    "word/media/image1.png",
)
WORD_NAMESPACE: Final[str] = (
    "http://schemas.openxmlformats.org/wordprocessingml/2006/main"
)
REL_NAMESPACE: Final[str] = (
    "http://schemas.openxmlformats.org/package/2006/relationships"
)


class DocxConformanceError(Exception):
    pass


def docx_case_bytes(case: dict[str, JsonValue]) -> bytes:
    output = io.BytesIO()
    try:
        with zipfile.ZipFile(
            output,
            "w",
            compression=zipfile.ZIP_DEFLATED,
            compresslevel=9,
            strict_timestamps=True,
        ) as archive:
            for name, value in case_parts(case):
                info = zipfile.ZipInfo(name, ENTRY_TIMESTAMP)
                info.compress_type = zipfile.ZIP_DEFLATED
                info.create_system = 3
                info.external_attr = ENTRY_MODE
                archive.writestr(
                    info, value, compress_type=zipfile.ZIP_DEFLATED, compresslevel=9
                )
    except (OSError, ValueError, zipfile.BadZipFile) as error:
        raise DocxConformanceError("DOCX package construction failed") from error
    value = output.getvalue()
    validate_docx_bytes(value)
    return value


def validate_docx_bytes(value: bytes) -> None:
    if not valid_ooxml_bytes(
        value,
        "word/document.xml",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml",
        f"{{{WORD_NAMESPACE}}}document",
    ):
        raise DocxConformanceError("DOCX package is not valid OOXML")
    try:
        with zipfile.ZipFile(io.BytesIO(value)) as archive:
            infos = archive.infolist()
            if tuple(info.filename for info in infos) != EXPECTED_PARTS:
                raise DocxConformanceError("DOCX package inventory differs")
            if any(
                info.date_time != ENTRY_TIMESTAMP
                or info.external_attr != ENTRY_MODE
                or info.compress_type != zipfile.ZIP_DEFLATED
                for info in infos
            ):
                raise DocxConformanceError("DOCX ZIP metadata differs")
            for name in EXPECTED_PARTS[:-1]:
                ElementTree.fromstring(archive.read(name))
            _validate_relationships(archive.read("word/_rels/document.xml.rels"))
    except (KeyError, zipfile.BadZipFile, ElementTree.ParseError) as error:
        raise DocxConformanceError("DOCX package validation failed") from error


def _validate_relationships(value: bytes) -> None:
    root = ElementTree.fromstring(value)
    relationships = tuple(root)
    expected = (
        ("rId1", "styles.xml"),
        ("rId2", "numbering.xml"),
        ("rId3", "header1.xml"),
        ("rId4", "footer1.xml"),
        ("rId5", "media/image1.png"),
    )
    actual = tuple(
        (item.attrib.get("Id"), item.attrib.get("Target")) for item in relationships
    )
    if root.tag != f"{{{REL_NAMESPACE}}}Relationships" or actual != expected:
        raise DocxConformanceError("DOCX relationships differ")
