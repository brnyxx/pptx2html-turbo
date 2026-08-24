from __future__ import annotations

import unittest
import zlib

from evaluate.multiformat_conformance_pdf import (
    PdfUnsupportedConstructError,
    canonicalize_pdf_bytes,
)
from evaluate.tests.test_multiformat_conformance_pdf import _type1_pdf
from evaluate.tests.test_multiformat_conformance_pdf_review import (
    _poppler_outputs,
    _write_pdf,
)


def _xmp_pdf(date: bytes, *, extra: bytes = b"") -> bytes:
    payload = (
        b'<x:xmpmeta xmlns:x="adobe:ns:meta/" xmlns:xmp="adobe:ns:meta/">'
        b"<xmp:CreateDate>" + date + b"</xmp:CreateDate></x:xmpmeta>"
    )
    encoded = zlib.compress(payload)
    metadata = (
        b"<< /Ty#70e /Meta#64ata /Sub#74ype /#58ML /Len#67th "
        + str(len(encoded)).encode()
        + b" /Fil#74er /Flate#44ecode "
        + extra
        + b" >>\nstream\n"
        + encoded
        + b"\nendstream"
    )
    objects = (
        (1, b"<< /Ty#70e /Cat#61log /Pages 2 0 R /Meta#64ata 4 0 R >>"),
        (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (3, b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>"),
        (4, metadata),
    )
    return _write_pdf(objects, b"/Root 1 0 R")


class MultiFormatConformancePdfDictionaryAuditTests(unittest.TestCase):
    def test_escaped_catalog_and_xmp_fields_normalize_deterministically(self) -> None:
        source = _xmp_pdf(b"2026-08-24T09:00:00Z")
        changed = _xmp_pdf(b"2026-08-24T10:00:00Z")
        canonical = canonicalize_pdf_bytes(source)
        info, source_text, source_png = _poppler_outputs(source)
        _, canonical_text, canonical_png = _poppler_outputs(canonical)

        self.assertIn(b"Pages:           1", info)
        self.assertEqual(canonical, canonicalize_pdf_bytes(changed))
        self.assertEqual(source_text, canonical_text)
        self.assertEqual(source_png, canonical_png)
        self.assertEqual(canonical, canonicalize_pdf_bytes(canonical))

    def test_xmp_duplicates_decode_parameters_and_malformed_names_fail(self) -> None:
        attacks = (
            (b"/Type /Metadata", "metadata-structure"),
            (b"/Decode#50arms << /Predictor 12 >>", "metadata-filter"),
            (b"/Len# 1", "metadata-structure"),
        )
        for extra, construct in attacks:
            with self.subTest(extra=extra):
                with self.assertRaises(PdfUnsupportedConstructError) as raised:
                    canonicalize_pdf_bytes(
                        _xmp_pdf(b"2026-08-24T09:00:00Z", extra=extra)
                    )
                self.assertEqual(raised.exception.construct, construct)

    def test_escaped_catalog_and_type1_fields_normalize_unique_id(self) -> None:
        first = canonicalize_pdf_bytes(_type1_pdf(4_100_001, escaped_fields=True))
        second = canonicalize_pdf_bytes(_type1_pdf(4_100_002, escaped_fields=True))

        self.assertEqual(first, second)
        self.assertEqual(first, canonicalize_pdf_bytes(first))

    def test_type1_duplicate_and_malformed_fields_fail_closed(self) -> None:
        attacks = (
            b"/Length#31 1",
            b"/Fil#74er /FlateDecode",
            b"/Length# 1",
        )
        for extra in attacks:
            with (
                self.subTest(extra=extra),
                self.assertRaises(PdfUnsupportedConstructError),
            ):
                canonicalize_pdf_bytes(_type1_pdf(4_100_001, font_extra=extra))
