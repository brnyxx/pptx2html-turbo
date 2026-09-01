"""Canonical form must be invariant under indirect-object renumbering."""

from __future__ import annotations

import re
import unittest
import zlib

from evaluate.multiformat_conformance_pdf import (
    PdfConformanceError,
    canonicalize_pdf_bytes,
)
from evaluate.multiformat_pdf_xref import parse_pdf_objects
from evaluate.tests.multiformat_pdf_permutation_fixture import (
    CJK_GRAPH,
    LATIN_GRAPH,
    build_pdf,
    face_swapped_order,
    natural_order,
    reversed_body_order,
    with_detached,
)


def _faces(pdf: bytes) -> list[bytes]:
    return sorted(re.findall(rb"/BaseFont\s*/([A-Za-z0-9+\-]+)", pdf))


def _subset_streams(pdf: bytes) -> list[bytes]:
    parsed = parse_pdf_objects(pdf)
    payloads = []
    for body in parsed.objects.values():
        match = re.search(rb"stream\r?\n", body)
        if match is None:
            continue
        end = body.rfind(b"\nendstream")
        payloads.append(zlib.decompress(body[match.end() : end]))
    return sorted(payloads)


class PdfCanonicalOrderTests(unittest.TestCase):
    """Both face pairings permute identically, so neither needs a host font."""

    def test_face_order_permutation_canonicalizes_identically(self) -> None:
        for label, graph in (("cjk", CJK_GRAPH), ("cjk-free", LATIN_GRAPH)):
            with self.subTest(graph=label):
                # Given the same document under two converter layouts.
                first = build_pdf(graph, natural_order(graph))
                second = build_pdf(graph, face_swapped_order(graph))
                self.assertNotEqual(first, second, "fixture must differ raw")

                # When
                left = canonicalize_pdf_bytes(first)
                right = canonicalize_pdf_bytes(second)

                # Then
                self.assertEqual(left, right)

    def test_every_layout_reaches_one_canonical_form(self) -> None:
        for label, graph in (("cjk", CJK_GRAPH), ("cjk-free", LATIN_GRAPH)):
            with self.subTest(graph=label):
                # Given three distinct numberings of one document.
                layouts = (
                    natural_order(graph),
                    face_swapped_order(graph),
                    reversed_body_order(graph),
                )
                raw = {build_pdf(graph, order) for order in layouts}
                self.assertEqual(len(raw), 3, "layouts must differ raw")

                # When
                canonical = {canonicalize_pdf_bytes(item) for item in raw}

                # Then
                self.assertEqual(len(canonical), 1)

    def test_canonical_form_preserves_faces_and_subset_streams(self) -> None:
        for label, graph in (("cjk", CJK_GRAPH), ("cjk-free", LATIN_GRAPH)):
            with self.subTest(graph=label):
                # Given
                source = build_pdf(graph, natural_order(graph))

                # When
                canonical = canonicalize_pdf_bytes(source)

                # Then no glyph data is lost or altered by renumbering.
                self.assertEqual(_faces(canonical), _faces(source))
                self.assertEqual(_subset_streams(canonical), _subset_streams(source))

    def test_canonical_form_is_idempotent(self) -> None:
        # Given
        source = build_pdf(CJK_GRAPH, face_swapped_order(CJK_GRAPH))

        # When
        once = canonicalize_pdf_bytes(source)
        twice = canonicalize_pdf_bytes(once)

        # Then
        self.assertEqual(once, twice)

    def test_canonical_objects_are_contiguous_from_the_catalog(self) -> None:
        # Given
        source = build_pdf(CJK_GRAPH, reversed_body_order(CJK_GRAPH))

        # When
        parsed = parse_pdf_objects(canonicalize_pdf_bytes(source))

        # Then numbering is dense and rooted, so it cannot encode input order.
        self.assertEqual(
            sorted(parsed.objects), list(range(1, len(parsed.objects) + 1))
        )
        self.assertEqual(parsed.root_id, 1)

    def test_unreachable_objects_are_preserved_and_ordered_stably(self) -> None:
        # Given one document emitted with two numberings, each carrying two
        # objects no reference reaches (an exporter may emit an unused font).
        graph = with_detached(
            CJK_GRAPH,
            (b"<</Type /Unused /Slot 1>>", b"<</Type /Unused /Slot 2>>"),
        )
        first = build_pdf(graph, natural_order(graph))
        # A layout that disagrees on both graph numbering and detached order.
        swapped = list(face_swapped_order(graph))
        left, right = swapped.index("detached_1"), swapped.index("detached_2")
        swapped[left], swapped[right] = swapped[right], swapped[left]
        second = build_pdf(graph, tuple(swapped))
        self.assertNotEqual(first, second, "fixture must differ raw")

        # When
        canonical = canonicalize_pdf_bytes(first)

        # Then the detached objects survive and both layouts agree.
        self.assertEqual(canonical, canonicalize_pdf_bytes(second))
        self.assertEqual(canonical.count(b"/Type /Unused"), 2)
        self.assertIn(b"/Slot 1", canonical)
        self.assertIn(b"/Slot 2", canonical)

    def test_indistinguishable_detached_objects_fail_closed(self) -> None:
        # Given two unreachable objects with identical content, whose relative
        # order cannot be derived from the document.
        graph = with_detached(CJK_GRAPH, (b"<</Type /Unused>>", b"<</Type /Unused>>"))
        source = build_pdf(graph, natural_order(graph))

        # When / Then the canonicalizer refuses to invent an order.
        with self.assertRaises(PdfConformanceError):
            _ = canonicalize_pdf_bytes(source)

    def test_dangling_reference_fails_closed(self) -> None:
        # Given a catalog pointing at an object that does not exist.
        source = build_pdf(CJK_GRAPH, natural_order(CJK_GRAPH))
        broken = source.replace(
            b"/Type /Catalog /Pages 2 0 R", b"/Type /Catalog /Pages 99 0 R", 1
        )
        self.assertNotEqual(broken, source)

        # When / Then
        with self.assertRaises(PdfConformanceError):
            _ = canonicalize_pdf_bytes(broken)
