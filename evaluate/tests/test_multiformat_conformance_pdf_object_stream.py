from __future__ import annotations

import unittest

from evaluate.multiformat_conformance_pdf import (
    PdfUnsupportedConstructError,
    canonicalize_pdf_bytes,
)
from evaluate.tests.test_multiformat_conformance_pdf import _xref_stream_pdf
from evaluate.tests.test_multiformat_conformance_pdf_review import _poppler_outputs


class MultiFormatConformancePdfObjectStreamTests(unittest.TestCase):
    def test_escaped_object_stream_fields_canonicalize_valid_pdf(self) -> None:
        source = _xref_stream_pdf(
            escaped_object_fields=True,
            object_stream_extra=b"/Not#4E 99",
        )
        canonical = canonicalize_pdf_bytes(source)
        info, source_text, source_png = _poppler_outputs(source)
        _, canonical_text, canonical_png = _poppler_outputs(canonical)

        self.assertIn(b"Pages:           1", info)
        self.assertNotIn(b"/Type /ObjStm", canonical)
        self.assertEqual(source_text, canonical_text)
        self.assertEqual(source_png, canonical_png)
        self.assertEqual(canonical, canonicalize_pdf_bytes(canonical))

    def test_object_stream_decode_parameters_are_typed_rejected(self) -> None:
        source = _xref_stream_pdf(
            object_stream_extra=b"/Decode#50arms << /Predictor 12 >>"
        )

        with self.assertRaises(PdfUnsupportedConstructError) as raised:
            canonicalize_pdf_bytes(source)
        self.assertEqual(raised.exception.construct, "object-filter")

    def test_object_stream_semantic_duplicates_fail_closed(self) -> None:
        duplicates = (
            b"/Ty#70e /ObjStm",
            b"/Len#67th 6 0 R",
            b"/Fil#74er /FlateDecode",
            b"/#4E 1",
            b"/Fi#72st 4",
        )
        for entry in duplicates:
            with self.subTest(entry=entry):
                with self.assertRaises(PdfUnsupportedConstructError) as raised:
                    canonicalize_pdf_bytes(_xref_stream_pdf(object_stream_extra=entry))
                self.assertEqual(raised.exception.construct, "object-structure")

    def test_malformed_object_names_fail_and_unrelated_names_do_not_alias(self) -> None:
        for entry in (b"/N# 1", b"/Fi#GGst 4", b"/Length# 6 0 R"):
            with self.subTest(entry=entry):
                with self.assertRaises(PdfUnsupportedConstructError) as raised:
                    canonicalize_pdf_bytes(_xref_stream_pdf(object_stream_extra=entry))
                self.assertEqual(raised.exception.construct, "object-structure")

        source = _xref_stream_pdf(object_stream_extra=b"/Not#4E 99")
        self.assertEqual(
            canonicalize_pdf_bytes(_xref_stream_pdf()),
            canonicalize_pdf_bytes(source),
        )
