from __future__ import annotations

from enum import StrEnum
from pathlib import Path
from typing import assert_never

from evaluate.multiformat_source_fixture import SourceFixtureError


class PdfSecurityFamily(StrEnum):
    MALFORMED_XREF = "malformed-xref"
    XREF_CYCLE = "xref-cycle"
    OBJECT_STREAM_BOMB = "object-stream-bomb"
    DEEP_PAGE_TREE = "deep-page-tree"
    EMBEDDED_FILE = "embedded-file"
    JAVASCRIPT_ACTION = "javascript-action"
    LAUNCH_ACTION = "launch-action"
    EXTERNAL_URI = "external-uri"
    OVERSIZED_IMAGE = "oversized-image"
    ENCRYPTED_DOCUMENT = "encrypted-document"


def write_pdf_security_fixture(path: Path, family: str) -> None:
    try:
        parsed_family = PdfSecurityFamily(family)
    except ValueError as error:
        raise SourceFixtureError(
            f"unsupported PDF security family: {family}"
        ) from error
    objects, trailer_extra = _objects_for(parsed_family)
    value, xref_offset = _pdf_bytes(objects, trailer_extra)
    match parsed_family:
        case PdfSecurityFamily.MALFORMED_XREF:
            value = value.replace(
                f"startxref\n{xref_offset}\n".encode(),
                f"startxref\n{xref_offset + 1}\n".encode(),
            )
        case PdfSecurityFamily.XREF_CYCLE:
            value, _ = _pdf_bytes(objects, f"/Prev {xref_offset}")
        case (
            PdfSecurityFamily.OBJECT_STREAM_BOMB
            | PdfSecurityFamily.DEEP_PAGE_TREE
            | PdfSecurityFamily.EMBEDDED_FILE
            | PdfSecurityFamily.JAVASCRIPT_ACTION
            | PdfSecurityFamily.LAUNCH_ACTION
            | PdfSecurityFamily.EXTERNAL_URI
            | PdfSecurityFamily.OVERSIZED_IMAGE
            | PdfSecurityFamily.ENCRYPTED_DOCUMENT
        ):
            pass
        case unreachable:
            assert_never(unreachable)
    path.write_bytes(value)


def _objects_for(family: PdfSecurityFamily) -> tuple[list[tuple[int, bytes]], str]:
    catalog_extra = b""
    page_extra = b""
    extras: list[tuple[int, bytes]] = []
    trailer_extra = ""
    match family:
        case PdfSecurityFamily.EMBEDDED_FILE:
            catalog_extra = (
                b"/Names << /EmbeddedFiles << /Names [(payload) 4 0 R] >> >>"
            )
            extras = [
                (4, b"<< /Type /Filespec /F (payload.bin) /EF << /F 5 0 R >> >>"),
                (
                    5,
                    b"<< /Type /EmbeddedFile /Length 3 >>\nstream\nOLE\nendstream",
                ),
            ]
        case PdfSecurityFamily.JAVASCRIPT_ACTION:
            catalog_extra = b"/OpenAction 4 0 R"
            extras = [(4, b"<< /S /JavaScript /JS (app.alert\\(1\\)) >>")]
        case PdfSecurityFamily.LAUNCH_ACTION:
            catalog_extra = b"/OpenAction 4 0 R"
            extras = [(4, b"<< /S /Launch /F (payload.exe) >>")]
        case PdfSecurityFamily.EXTERNAL_URI:
            catalog_extra = b"/OpenAction 4 0 R"
            extras = [(4, b"<< /S /URI /URI (https://example.invalid/) >>")]
        case PdfSecurityFamily.OBJECT_STREAM_BOMB:
            catalog_extra = b"/OpenAction 4 0 R"
            extras = [
                (
                    4,
                    (
                        b"<< /Type /ObjStm /N 100001 /First 0 /Length 1 >>"
                        b"\nstream\nx\nendstream"
                    ),
                )
            ]
        case PdfSecurityFamily.OVERSIZED_IMAGE:
            page_extra = b"/Resources << /XObject << /Im0 4 0 R >> >>"
            extras = [
                (
                    4,
                    (
                        b"<< /Type /XObject /Subtype /Image /Width 200000 "
                        b"/Height 200000 /ColorSpace /DeviceRGB "
                        b"/BitsPerComponent 8 /Length 1 >>\nstream\nx\nendstream"
                    ),
                )
            ]
        case PdfSecurityFamily.ENCRYPTED_DOCUMENT:
            trailer_extra = "/Encrypt 4 0 R"
            extras = [(4, b"<< /Filter /Standard /V 2 /R 3 /Length 128 >>")]
        case PdfSecurityFamily.DEEP_PAGE_TREE:
            return _deep_page_objects(), trailer_extra
        case PdfSecurityFamily.MALFORMED_XREF | PdfSecurityFamily.XREF_CYCLE:
            pass
        case unreachable:
            assert_never(unreachable)
    objects = [
        (1, b"<< /Type /Catalog /Pages 2 0 R " + catalog_extra + b" >>"),
        (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (
            3,
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] "
            + page_extra
            + b" >>",
        ),
        *extras,
    ]
    return objects, trailer_extra


def _deep_page_objects() -> list[tuple[int, bytes]]:
    last_pages = 68
    objects: list[tuple[int, bytes]] = [(1, b"<< /Type /Catalog /Pages 2 0 R >>")]
    for object_id in range(2, last_pages):
        objects.append(
            (
                object_id,
                (f"<< /Type /Pages /Kids [{object_id + 1} 0 R] /Count 1 >>").encode(),
            )
        )
    objects.append(
        (
            last_pages,
            (
                f"<< /Type /Page /Parent {last_pages - 1} 0 R "
                "/MediaBox [0 0 612 792] >>"
            ).encode(),
        )
    )
    return objects


def _pdf_bytes(
    objects: list[tuple[int, bytes]],
    trailer_extra: str,
) -> tuple[bytes, int]:
    value = bytearray(b"%PDF-1.7\n")
    offsets: dict[int, int] = {}
    for object_id, body in objects:
        offsets[object_id] = len(value)
        value.extend(f"{object_id} 0 obj\n".encode())
        value.extend(body)
        value.extend(b"\nendobj\n")
    xref_offset = len(value)
    size = max(offsets) + 1
    value.extend(f"xref\n0 {size}\n".encode())
    value.extend(b"0000000000 65535 f \n")
    for object_id in range(1, size):
        offset = offsets.get(object_id)
        if offset is None:
            value.extend(b"0000000000 00000 f \n")
        else:
            value.extend(f"{offset:010d} 00000 n \n".encode())
    value.extend(
        (
            f"trailer\n<< /Size {size} /Root 1 0 R {trailer_extra} >>\n"
            f"startxref\n{xref_offset}\n%%EOF\n"
        ).encode()
    )
    return bytes(value), xref_offset
