from __future__ import annotations

import hashlib
import struct
import zipfile
from pathlib import Path

OLE_SIGNATURE = bytes.fromhex("d0cf11e0a1b11ae1")
FREE_SECTOR = 0xFFFFFFFF
END_OF_CHAIN = 0xFFFFFFFE
FAT_SECTOR = 0xFFFFFFFD
OOXML_PARTS = {
    "docx": (
        "word/document.xml",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml",
        "http://schemas.openxmlformats.org/wordprocessingml/2006/main",
        "document",
    ),
    "xlsx": (
        "xl/workbook.xml",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml",
        "http://schemas.openxmlformats.org/spreadsheetml/2006/main",
        "workbook",
    ),
    "pptx": (
        "ppt/presentation.xml",
        "application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml",
        "http://schemas.openxmlformats.org/presentationml/2006/main",
        "presentation",
    ),
}
LEGACY_STREAMS = {
    "doc": "WordDocument",
    "xls": "Workbook",
    "ppt": "PowerPoint Document",
}


class SourceFixtureError(Exception):
    pass


def write_positive_source(path: Path, document_format: str, marker: str) -> None:
    ooxml = OOXML_PARTS.get(document_format)
    if ooxml is not None:
        _write_ooxml(path, marker, *ooxml)
        return
    legacy_stream = LEGACY_STREAMS.get(document_format)
    if legacy_stream is not None:
        _write_cfb(path, legacy_stream, marker)
        return
    if document_format == "pdf":
        _write_pdf(path, marker)
        return
    raise SourceFixtureError(f"unsupported fixture format: {document_format}")


def write_ambiguous_legacy_source(path: Path, document_format: str) -> None:
    stream_name = LEGACY_STREAMS[document_format]
    competing = "Workbook" if document_format != "xls" else "WordDocument"
    _write_cfb(path, stream_name, "ambiguous", competing)


def write_ambiguous_ooxml_source(path: Path) -> None:
    content_types = (
        '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
        '<Override PartName="/word/document.xml" '
        'ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>'
        '<Override PartName="/xl/workbook.xml" '
        'ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>'
        "</Types>"
    )
    relationships = (
        '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
        '<Relationship Id="rId1" '
        'Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" '
        'Target="word/document.xml"/>'
        '<Relationship Id="rId2" '
        'Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" '
        'Target="xl/workbook.xml"/>'
        "</Relationships>"
    )
    with zipfile.ZipFile(path, "w") as archive:
        for name, content in [
            ("[Content_Types].xml", content_types),
            ("_rels/.rels", relationships),
            (
                "word/document.xml",
                '<document xmlns="http://schemas.openxmlformats.org/wordprocessingml/2006/main"/>',
            ),
            (
                "xl/workbook.xml",
                '<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"/>',
            ),
        ]:
            entry = zipfile.ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
            archive.writestr(entry, content)


def _write_ooxml(
    path: Path,
    marker: str,
    main_part: str,
    content_type: str,
    namespace: str,
    root_name: str,
) -> None:
    content_types = (
        '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
        f'<Override PartName="/{main_part}" ContentType="{content_type}"/>'
        "</Types>"
    )
    relationships = (
        '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
        '<Relationship Id="rId1" '
        'Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" '
        f'Target="{main_part}"/>'
        "</Relationships>"
    )
    main_xml = f'<{root_name} xmlns="{namespace}"><!--{marker}--></{root_name}>'
    with zipfile.ZipFile(path, "w") as archive:
        for name, content in [
            ("[Content_Types].xml", content_types),
            ("_rels/.rels", relationships),
            (main_part, main_xml),
        ]:
            entry = zipfile.ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
            archive.writestr(entry, content)


def _write_cfb(
    path: Path,
    stream_name: str,
    marker: str,
    competing_stream: str | None = None,
) -> None:
    header = bytearray(512)
    header[:8] = OLE_SIGNATURE
    struct.pack_into("<HHHH", header, 24, 0x003E, 3, 0xFFFE, 9)
    struct.pack_into("<H", header, 32, 6)
    struct.pack_into("<I", header, 44, 1)
    struct.pack_into("<I", header, 48, 0)
    struct.pack_into("<I", header, 56, 4096)
    struct.pack_into("<I", header, 60, END_OF_CHAIN)
    struct.pack_into("<I", header, 64, 0)
    struct.pack_into("<I", header, 68, END_OF_CHAIN)
    struct.pack_into("<I", header, 72, 0)
    for index in range(109):
        struct.pack_into("<I", header, 76 + index * 4, FREE_SECTOR)
    struct.pack_into("<I", header, 76, 9)

    directory = bytearray(512)
    directory[:128] = _directory_entry("Root Entry", 5, child=1)
    directory[128:256] = _directory_entry(
        stream_name,
        2,
        right=2,
        start_sector=1,
        stream_size=4096,
    )
    marker_name = "M" + hashlib.sha256(marker.encode()).hexdigest()[:20]
    if competing_stream is None:
        directory[256:384] = _directory_entry(marker_name, 2)
    else:
        directory[256:384] = _directory_entry(
            competing_stream,
            2,
            right=3,
            start_sector=1,
            stream_size=4096,
        )
        directory[384:512] = _directory_entry(marker_name, 2)

    stream = bytearray(4096)
    stream[: len(marker)] = marker.encode()
    fat = bytearray(b"\xff" * 512)
    struct.pack_into("<I", fat, 0, END_OF_CHAIN)
    for sector_id in range(1, 8):
        struct.pack_into("<I", fat, sector_id * 4, sector_id + 1)
    struct.pack_into("<I", fat, 8 * 4, END_OF_CHAIN)
    struct.pack_into("<I", fat, 9 * 4, FAT_SECTOR)
    path.write_bytes(bytes(header + directory + stream + fat))


def _write_pdf(path: Path, marker: str) -> None:
    value = bytearray(f"%PDF-1.7\n% {marker}\n".encode())
    offsets = [0]
    for object_id, body in [
        (1, "<< /Type /Catalog /Pages 2 0 R >>"),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (3, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>"),
    ]:
        offsets.append(len(value))
        value.extend(f"{object_id} 0 obj\n{body}\nendobj\n".encode())
    xref_offset = len(value)
    value.extend(b"xref\n0 4\n")
    value.extend(b"0000000000 65535 f \n")
    for offset in offsets[1:]:
        value.extend(f"{offset:010d} 00000 n \n".encode())
    value.extend(
        (
            f"trailer\n<< /Size 4 /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n"
        ).encode()
    )
    path.write_bytes(value)


def _directory_entry(
    name: str,
    object_type: int,
    *,
    child: int = FREE_SECTOR,
    right: int = FREE_SECTOR,
    start_sector: int = END_OF_CHAIN,
    stream_size: int = 0,
) -> bytes:
    entry = bytearray(128)
    encoded = name.encode("utf-16le") + b"\x00\x00"
    entry[: len(encoded)] = encoded
    struct.pack_into("<H", entry, 64, len(encoded))
    entry[66] = object_type
    entry[67] = 1
    struct.pack_into("<III", entry, 68, FREE_SECTOR, right, child)
    struct.pack_into("<I", entry, 116, start_sector)
    struct.pack_into("<Q", entry, 120, stream_size)
    return bytes(entry)
