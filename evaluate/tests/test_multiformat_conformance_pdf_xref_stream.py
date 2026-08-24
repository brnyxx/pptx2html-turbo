from __future__ import annotations

import unittest

from evaluate.multiformat_conformance_pdf import (
    PdfUnsupportedConstructError,
    canonicalize_pdf_bytes,
)
from evaluate.tests.test_multiformat_conformance_pdf import _xref_stream_pdf
from evaluate.tests.test_multiformat_conformance_pdf_review import _poppler_outputs


def _xref_replace(value: bytes, old: bytes, new: bytes) -> bytes:
    prefix, separator, xref = value.partition(b"7 0 obj\n")
    if not separator or old not in xref:
        raise AssertionError("xref-stream fixture field is missing")
    return prefix + separator + xref.replace(old, new, 1)


def _xref_add(value: bytes, entry: bytes) -> bytes:
    return _xref_replace(value, b"/Root 1 0 R >>", b"/Root 1 0 R " + entry + b" >>")


def _escaped_xref_stream_pdf() -> bytes:
    value = _xref_stream_pdf()
    replacements = (
        (b"/Type /XRef", b"/Ty#70e /XRef"),
        (b"/Length ", b"/Len#67th "),
        (b"/Filter /FlateDecode", b"/Fil#74er /FlateDecode"),
        (b"/Size 8", b"/Si#7Ae 8"),
        (b"/W [1 4 2]", b"/#57 [1 4 2]"),
        (b"/Root 1 0 R", b"/Ro#6ft 1 0 R"),
    )
    for old, new in replacements:
        value = _xref_replace(value, old, new)
    return _xref_replace(
        value,
        b"/Ro#6ft 1 0 R >>",
        b"/In#64ex [0 8] /Ro#6ft 1 0 R /I#44 [<AAAAAAAA><AAAAAAAA>] "
        b"/DocCheck#73um /ABCDEF /NotSi#7Ae 99 >>",
    )


class MultiFormatConformancePdfXrefStreamTests(unittest.TestCase):
    def test_escaped_consumed_fields_canonicalize_valid_xref_stream(self) -> None:
        source = _escaped_xref_stream_pdf()
        canonical = canonicalize_pdf_bytes(source)
        info, source_text, source_png = _poppler_outputs(source)
        _, canonical_text, canonical_png = _poppler_outputs(canonical)

        self.assertIn(b"Pages:           1", info)
        self.assertIn(b"/ID [<00000000><00000000>]", canonical)
        self.assertIn(b"/DocChecksum /000000", canonical)
        self.assertEqual(source_text, canonical_text)
        self.assertEqual(source_png, canonical_png)
        self.assertEqual(canonical, canonicalize_pdf_bytes(canonical))

    def test_escaped_unsupported_xref_fields_remain_rejected(self) -> None:
        source = _xref_stream_pdf()
        attacks = (
            (b"/Encr#79pt 8 0 R", "encryption"),
            (b"/Pr#65v 12", "incremental-update"),
            (b"/XRef#53tm 12", "hybrid-xref"),
        )
        for entry, construct in attacks:
            with self.subTest(entry=entry):
                with self.assertRaises(PdfUnsupportedConstructError) as raised:
                    canonicalize_pdf_bytes(_xref_add(source, entry))
                self.assertEqual(raised.exception.construct, construct)

    def test_decode_parameters_are_typed_rejected(self) -> None:
        source = _xref_add(
            _xref_stream_pdf(), b"/Decode#50arms << /Predictor 12 /Columns 7 >>"
        )

        with self.assertRaises(PdfUnsupportedConstructError) as raised:
            canonicalize_pdf_bytes(source)
        self.assertEqual(raised.exception.construct, "xref-filter")

    def test_plain_and_escaped_xref_field_duplicates_fail_closed(self) -> None:
        source = _xref_stream_pdf()
        duplicates = (
            b"/Ty#70e /XRef",
            b"/Len#67th 1",
            b"/Fil#74er /FlateDecode",
            b"/Si#7Ae 8",
            b"/#57 [1 4 2]",
            b"/Index [0 8] /In#64ex [0 8]",
            b"/Ro#6ft 1 0 R",
        )
        for entry in duplicates:
            with self.subTest(entry=entry):
                with self.assertRaises(PdfUnsupportedConstructError) as raised:
                    canonicalize_pdf_bytes(_xref_add(source, entry))
                self.assertEqual(raised.exception.construct, "xref-structure")

    def test_malformed_xref_names_fail_and_unrelated_names_do_not_alias(self) -> None:
        source = _xref_stream_pdf()
        for entry in (b"/Si#7 8", b"/Len#GGth 1", b"/Filter# /FlateDecode"):
            with self.subTest(entry=entry):
                with self.assertRaises(PdfUnsupportedConstructError) as raised:
                    canonicalize_pdf_bytes(_xref_add(source, entry))
                self.assertEqual(raised.exception.construct, "xref-structure")

        unrelated = _xref_add(source, b"/NotSi#7Ae 99")
        self.assertEqual(
            canonicalize_pdf_bytes(source), canonicalize_pdf_bytes(unrelated)
        )
