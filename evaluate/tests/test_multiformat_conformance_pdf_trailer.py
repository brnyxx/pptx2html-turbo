from __future__ import annotations

import unittest

from evaluate.multiformat_conformance_pdf import (
    PdfUnsupportedConstructError,
    canonicalize_pdf_bytes,
)
from evaluate.tests.test_multiformat_conformance_pdf_review import (
    _poppler_outputs,
    _visible_pdf,
)


def _trailer_replace(value: bytes, old: bytes, new: bytes) -> bytes:
    prefix, separator, trailer = value.partition(b"trailer\n")
    if not separator or old not in trailer:
        raise AssertionError("trailer fixture field is missing")
    return prefix + separator + trailer.replace(old, new, 1)


def _trailer_add(value: bytes, entry: bytes) -> bytes:
    return _trailer_replace(value, b"/Root 1 0 R", b"/Root 1 0 R " + entry)


class MultiFormatConformancePdfTrailerTests(unittest.TestCase):
    def test_escaped_root_info_and_size_resolve_and_normalize(self) -> None:
        source = _visible_pdf(b"D:20260824090000Z", b"AAAAAAAA")
        escaped = _trailer_replace(source, b"/Size 8", b"/Si#7Ae 8")
        escaped = _trailer_replace(escaped, b"/Root 1 0 R", b"/R#6fot 1 0 R")
        escaped = _trailer_replace(escaped, b"/Info 6 0 R", b"/I#6Efo 6 0 R")

        canonical = canonicalize_pdf_bytes(escaped)
        info, source_text, source_png = _poppler_outputs(escaped)
        _, canonical_text, canonical_png = _poppler_outputs(canonical)

        self.assertIn(b"Title:           upstream report", info)
        self.assertIn(b"/CreationDate (" + b"0" * 17 + b")", canonical)
        self.assertEqual(canonical, canonicalize_pdf_bytes(source))
        self.assertEqual(source_text, canonical_text)
        self.assertEqual(source_png, canonical_png)
        self.assertEqual(canonical, canonicalize_pdf_bytes(canonical))

    def test_escaped_id_and_checksum_normalize_deterministically(self) -> None:
        first = _visible_pdf(b"D:20260824090000Z", b"AAAAAAAA")
        second = _visible_pdf(b"D:20260824090000Z", b"BBBBBBBB")
        values = ((first, b"ABCDEF"), (second, b"FEDCBA"))
        canonical: list[bytes] = []
        for source, checksum in values:
            escaped = _trailer_replace(source, b"/ID", b"/I#44")
            escaped = _trailer_replace(
                escaped, b"/DocChecksum /ABCDEF", b"/DocCheck#73um /" + checksum
            )
            canonical.append(canonicalize_pdf_bytes(escaped))

        self.assertEqual(canonical[0], canonical[1])
        self.assertIn(b"/ID [<00000000><00000000>]", canonical[0])
        self.assertIn(b"/DocChecksum /000000", canonical[0])

    def test_escaped_unsupported_trailer_names_remain_rejected(self) -> None:
        source = _visible_pdf(b"D:20260824090000Z", b"AAAAAAAA")
        attacks = (
            (b"/Encr#79pt 8 0 R", "encryption"),
            (b"/Pr#65v 12", "incremental-update"),
            (b"/XRef#53tm 12", "hybrid-xref"),
        )
        for entry, construct in attacks:
            with self.subTest(entry=entry):
                with self.assertRaises(PdfUnsupportedConstructError) as raised:
                    canonicalize_pdf_bytes(_trailer_add(source, entry))
                self.assertEqual(raised.exception.construct, construct)

    def test_semantic_duplicate_trailer_names_fail_closed(self) -> None:
        source = _visible_pdf(b"D:20260824090000Z", b"AAAAAAAA")
        duplicates = (
            b"/R#6fot 1 0 R",
            b"/I#6Efo 6 0 R",
            b"/Si#7Ae 8",
            b"/I#44 [<AAAAAAAA><AAAAAAAA>]",
            b"/DocCheck#73um /ABCDEF",
            b"/Encrypt 8 0 R /Encr#79pt 8 0 R",
            b"/Prev 12 /Pr#65v 12",
            b"/XRefStm 12 /XRef#53tm 12",
        )
        for entries in duplicates:
            with self.subTest(entries=entries):
                with self.assertRaises(PdfUnsupportedConstructError) as raised:
                    canonicalize_pdf_bytes(_trailer_add(source, entries))
                self.assertEqual(raised.exception.construct, "trailer-structure")

    def test_malformed_escapes_fail_and_unrelated_names_do_not_alias(self) -> None:
        source = _visible_pdf(b"D:20260824090000Z", b"AAAAAAAA")
        for entry in (b"/I#6 6 0 R", b"/Ro#GGt 1 0 R", b"/Size# 8"):
            with self.subTest(entry=entry):
                with self.assertRaises(PdfUnsupportedConstructError) as raised:
                    canonicalize_pdf_bytes(_trailer_add(source, entry))
                self.assertEqual(raised.exception.construct, "trailer-structure")

        unrelated = _trailer_add(source, b"/NotI#6Efo 99 0 R")
        self.assertEqual(
            canonicalize_pdf_bytes(source), canonicalize_pdf_bytes(unrelated)
        )
