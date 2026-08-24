from __future__ import annotations

import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

from evaluate.multiformat_conformance_pdf import (
    ParsedPdfObjects,
    PdfConformanceError,
    PdfUnsupportedConstructError,
    canonicalize_pdf_bytes,
)
from evaluate.multiformat_pdf_writer import canonicalize_pdf_metadata

PDFINFO = Path(shutil.which("pdfinfo") or "pdfinfo")
PDFTOTEXT = Path(shutil.which("pdftotext") or "pdftotext")
PDFTOCAIRO = Path(shutil.which("pdftocairo") or "pdftocairo")
VISIBLE_TEXT = b"Visible /CreationDate (SECRET) text"


def _write_pdf(objects: tuple[tuple[int, bytes], ...], trailer: bytes) -> bytes:
    value = bytearray(b"%PDF-1.7\n")
    offsets: dict[int, int] = {}
    for object_id, body in objects:
        offsets[object_id] = len(value)
        value.extend(f"{object_id} 0 obj\n".encode() + body + b"\nendobj\n")
    size = max(offsets) + 1
    xref_offset = len(value)
    value.extend(f"xref\n0 {size}\n0000000000 65535 f \n".encode())
    for object_id in range(1, size):
        offset = offsets.get(object_id)
        entry = (
            b"0000000000 00000 f \n"
            if offset is None
            else f"{offset:010d} 00000 n \n".encode()
        )
        value.extend(entry)
    value.extend(
        b"trailer\n<< /Size "
        + str(size).encode()
        + b" "
        + trailer
        + f" >>\nstartxref\n{xref_offset}\n%%EOF\n".encode()
    )
    return bytes(value)


def _info_dictionary(date: bytes, comment_end: bytes = b"") -> bytes:
    first_comment = b" % valid CR comment" + comment_end if comment_end else b""
    second_comment = b" % ([]) <<>> /stream" + comment_end if comment_end else b""
    return (
        b"<< /Title (upstream report) /Subject (escaped \\(stream\\) and "
        b"(nested stream))"
        + first_comment
        + b" /CreationDate ("
        + date
        + b")"
        + second_comment
        + b" /ModDate ("
        + date
        + b") >>"
    )


def _escaped_info(date: bytes, creation_key: bytes, modified_key: bytes) -> bytes:
    return (
        b"<< /Title (upstream report) /NotCreation#44ate (KEEP) /"
        + creation_key
        + b" ("
        + date
        + b") /"
        + modified_key
        + b" ("
        + date
        + b") >>"
    )


def _page_pdf(info: bytes, document_id: bytes, info_reference: int = 6) -> bytes:
    content = b"BT /F1 18 Tf 72 720 Td (" + VISIBLE_TEXT + b") Tj ET"
    objects = (
        (1, b"<< /Type /Catalog /Pages 2 0 R >>"),
        (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (
            3,
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>",
        ),
        (
            4,
            f"<< /Length {len(content)} >>\nstream\n".encode()
            + content
            + b"\nendstream",
        ),
        (5, b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>"),
        (6, info),
        (7, b"<< /CreationDate (UNRELATED) >>"),
    )
    trailer = (
        b"/Root 1 0 R /Info "
        + str(info_reference).encode()
        + b" 0 R /ID [<"
        + document_id
        + b"><"
        + document_id
        + b">] /DocChecksum /ABCDEF"
    )
    return _write_pdf(objects, trailer)


def _visible_pdf(date: bytes, document_id: bytes, info_reference: int = 6) -> bytes:
    return _page_pdf(_info_dictionary(date), document_id, info_reference)


def _poppler_outputs(value: bytes) -> tuple[bytes, bytes, bytes]:
    with tempfile.TemporaryDirectory() as temp_dir:
        root = Path(temp_dir)
        source = root / "source.pdf"
        source.write_bytes(value)
        info = subprocess.run([PDFINFO, source], check=True, capture_output=True).stdout
        text = subprocess.run(
            [PDFTOTEXT, source, "-"], check=True, capture_output=True
        ).stdout
        subprocess.run(
            [PDFTOCAIRO, "-f", "1", "-singlefile", "-png", source, root / "page"],
            check=True,
        )
        return info, text, (root / "page.png").read_bytes()


def _info_stream_pdf() -> bytes:
    objects = (
        (1, b"<< /Type /Catalog /Pages 2 0 R >>"),
        (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (3, b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>"),
        (6, b"<< /Length 4 >>\nstream\ndata\nendstream"),
    )
    return _write_pdf(objects, b"/Root 1 0 R /Info 6 0 R")


def _metadata_pdf(
    metadata_filter: bytes,
    catalog_metadata: bytes = b"/Metadata 4 0 R",
) -> bytes:
    payload = b'<x:xmpmeta xmlns:x="adobe:ns:meta/"><xmp:CreateDate>2026-08-24T09:00:00Z</xmp:CreateDate></x:xmpmeta>'
    metadata = (
        b"<< /Type /Metadata /Subtype /XML "
        + metadata_filter
        + f" /Length {len(payload)} >>\nstream\n".encode()
        + payload
        + b"\nendstream"
    )
    objects = (
        (1, b"<< /Type /Catalog /Pages 2 0 R " + catalog_metadata + b" >>"),
        (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (3, b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>"),
        (4, metadata),
    )
    return _write_pdf(objects, b"/Root 1 0 R")


class MultiFormatConformancePdfReviewTests(unittest.TestCase):
    def test_visible_date_like_text_survives_text_and_render_canonicalization(
        self,
    ) -> None:
        source = _visible_pdf(b"D:20260824090000Z", b"AAAAAAAA")
        canonical = canonicalize_pdf_bytes(source)
        second = canonicalize_pdf_bytes(_visible_pdf(b"D:20260824100000Z", b"BBBBBBBB"))
        repeated = canonicalize_pdf_bytes(canonical)

        _, source_text, source_png = _poppler_outputs(source)
        info, canonical_text, canonical_png = _poppler_outputs(canonical)

        self.assertEqual(source_text, canonical_text)
        self.assertEqual(source_png, canonical_png)
        self.assertIn(b"Title:           upstream report", info)
        self.assertIn(b"SECRET", source_text)
        self.assertIn(b"/Title (upstream report)", canonical)
        self.assertIn(b"escaped \\(stream\\) and (nested stream)", canonical)
        self.assertIn(b"/CreationDate (" + b"0" * 17 + b")", canonical)
        self.assertIn(b"/CreationDate (UNRELATED)", canonical)
        self.assertEqual(canonical, second)
        self.assertEqual(canonical, repeated)

    def test_escaped_info_date_names_normalize_deterministically(self) -> None:
        key_pairs = (
            (b"Creation#44ate", b"Mod#44ate"),
            (b"Cre#61tionDate", b"M#6fdDate"),
        )
        for creation_key, modified_key in key_pairs:
            with self.subTest(keys=(creation_key, modified_key)):
                source = _page_pdf(
                    _escaped_info(b"D:20260824090000Z", creation_key, modified_key),
                    b"EEEEEEEE",
                )
                changed = _page_pdf(
                    _escaped_info(b"D:20260824100000Z", creation_key, modified_key),
                    b"EEEEEEEE",
                )
                canonical = canonicalize_pdf_bytes(source)
                info, source_text, source_png = _poppler_outputs(source)
                _, canonical_text, canonical_png = _poppler_outputs(canonical)

                self.assertIn(b"Title:           upstream report", info)
                self.assertIn(b"/" + creation_key + b" (" + b"0" * 17 + b")", canonical)
                self.assertIn(b"/NotCreation#44ate (KEEP)", canonical)
                self.assertEqual(canonical, canonicalize_pdf_bytes(changed))
                self.assertEqual(source_text, canonical_text)
                self.assertEqual(source_png, canonical_png)
                self.assertEqual(canonical, canonicalize_pdf_bytes(canonical))

    def test_semantically_duplicate_info_date_names_fail_closed(self) -> None:
        info = (
            b"<< /CreationDate (D:20260824090000Z) "
            b"/Creation#44ate (D:20260824100000Z) >>"
        )

        with self.assertRaises(PdfUnsupportedConstructError) as raised:
            canonicalize_pdf_bytes(_page_pdf(info, b"FFFFFFFF"))
        self.assertEqual(raised.exception.construct, "metadata-structure")

    def test_malformed_info_name_escapes_fail_closed(self) -> None:
        for name in (b"Creation#4", b"Creation#GGate", b"CreationDate#"):
            with self.subTest(name=name):
                with self.assertRaises(PdfUnsupportedConstructError) as raised:
                    canonicalize_pdf_bytes(
                        _page_pdf(b"<< /" + name + b" (value) >>", b"FFFFFFFF")
                    )
                self.assertEqual(raised.exception.construct, "metadata-structure")

    def test_info_comments_support_cr_lf_and_crlf(self) -> None:
        for ending in (b"\r", b"\n", b"\r\n"):
            with self.subTest(ending=ending):
                source = _page_pdf(
                    _info_dictionary(b"D:20260824090000Z", ending), b"CCCCCCCC"
                )
                canonical = canonicalize_pdf_bytes(source)
                info, source_text, source_png = _poppler_outputs(source)
                _, canonical_text, canonical_png = _poppler_outputs(canonical)

                self.assertIn(b"Title:           upstream report", info)
                self.assertIn(b"% valid CR comment" + ending, canonical)
                self.assertIn(b"/CreationDate (" + b"0" * 17 + b")", canonical)
                self.assertEqual(source_text, canonical_text)
                self.assertEqual(source_png, canonical_png)
                self.assertEqual(canonical, canonicalize_pdf_bytes(canonical))

    def test_info_comment_at_eof_fails_closed(self) -> None:
        parsed = ParsedPdfObjects(b"%PDF-1.7\n", {}, 1, 6, None, None)
        objects = {
            1: b"1 0 obj\n<< /Type /Catalog >>\nendobj",
            6: b"6 0 obj\n<< /Title (upstream report) >> % comment at EOF",
        }

        with self.assertRaises(PdfUnsupportedConstructError) as raised:
            canonicalize_pdf_metadata(parsed, objects)
        self.assertEqual(raised.exception.construct, "metadata-structure")

    def test_genuine_info_stream_fails_with_typed_error(self) -> None:
        with self.assertRaises(PdfUnsupportedConstructError) as raised:
            canonicalize_pdf_bytes(_info_stream_pdf())
        self.assertEqual(raised.exception.construct, "metadata-structure")

    def test_dangling_retained_info_reference_fails_before_output(self) -> None:
        source = _visible_pdf(b"D:20260824090000Z", b"AAAAAAAA", 99)

        with self.assertRaisesRegex(PdfConformanceError, "Info.*unresolved"):
            canonicalize_pdf_bytes(source)

    def test_unsupported_xmp_filter_fails_with_typed_error(self) -> None:
        source = _metadata_pdf(b"/Filter /ASCIIHexDecode")

        with self.assertRaises(PdfUnsupportedConstructError) as raised:
            canonicalize_pdf_bytes(source)
        self.assertEqual(raised.exception.construct, "metadata-filter")

    def test_ambiguous_xmp_reference_fails_with_typed_error(self) -> None:
        source = _metadata_pdf(b"", b"/Metadata [4 0 R]")

        with self.assertRaises(PdfUnsupportedConstructError) as raised:
            canonicalize_pdf_bytes(source)
        self.assertEqual(raised.exception.construct, "metadata-structure")
