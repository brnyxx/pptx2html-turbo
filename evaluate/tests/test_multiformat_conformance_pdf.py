from __future__ import annotations

import tempfile
import unittest
import zlib
from pathlib import Path

from evaluate.multiformat_conformance_pdf import normalize_pdf_bytes, pdf_case_html
from evaluate.multiformat_corpus_sources import validate_source
from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_pdf_link_annotation import add_link_annotation
from evaluate.multiformat_pdf_writer import canonicalize_pdf_bytes
from evaluate.multiformat_schema import sha256_file
from evaluate.tests.multiformat_source_fixture import write_positive_source


def _xref_stream_pdf() -> bytes:
    value = bytearray(b"%PDF-1.5\n")
    offsets = [0]
    for object_id, body in [
        (1, b"<< /Type /Catalog /Pages 2 0 R >>"),
        (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (3, b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>"),
    ]:
        offsets.append(len(value))
        value.extend(f"{object_id} 0 obj\n".encode() + body + b"\nendobj\n")
    object_stream_body = b"5 0 << /CreationDate (D:20260821120319+09'00) >>"
    object_stream_data = zlib.compress(object_stream_body)
    offsets.append(len(value))
    value.extend(
        b"4 0 obj\n<< /Type /ObjStm /Length 6 0 R /N 1 /First 4 "
        b"/Filter /FlateDecode >>\nstream\n"
    )
    value.extend(object_stream_data + b"\nendstream\nendobj\n")
    offsets.append(len(value))
    value.extend(f"6 0 obj\n{len(object_stream_data)}\nendobj\n".encode())
    xref_offset = len(value)
    entries = [
        b"\x00\x00\x00\x00\x00\xff\xff",
        *[b"\x01" + offset.to_bytes(4, "big") + b"\x00\x00" for offset in offsets[1:5]],
        b"\x02\x00\x00\x00\x04\x00\x00",
        b"\x01" + offsets[5].to_bytes(4, "big") + b"\x00\x00",
        b"\x01" + xref_offset.to_bytes(4, "big") + b"\x00\x00",
    ]
    xref_data = b"".join(entries)
    compressed = zlib.compress(xref_data)
    value.extend(
        (
            f"7 0 obj\n<< /Type /XRef /Length {len(compressed)} "
            "/Filter /FlateDecode /Size 8 /W [1 4 2] /Root 1 0 R >>\nstream\n"
        ).encode()
    )
    value.extend(compressed + b"\nendstream\nendobj\n")
    value.extend(f"startxref\n{xref_offset}\n%%EOF\n".encode())
    return bytes(value)


def _type1_pdf(unique_id: int) -> bytes:
    clear = b"%!FontType1-1.0: Deterministic 1.0\neexec\n"
    encrypted = _type1_encrypt(f"    /UniqueID {unique_id} def\n".encode())
    trailer = b"\ncleartomark\n"
    program = clear + encrypted + trailer
    compressed = zlib.compress(program)
    objects = [
        (1, b"<< /Type /Catalog /Pages 2 0 R >>"),
        (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (3, b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>"),
        (
            4,
            (
                f"<< /Length 5 0 R /Filter /FlateDecode "
                f"/Length1 {len(clear)} /Length2 {len(encrypted)} "
                f"/Length3 {len(trailer)} >>\nstream\n"
            ).encode()
            + compressed
            + b"\nendstream",
        ),
        (5, str(len(compressed)).encode()),
    ]
    value = bytearray(b"%PDF-1.5\n")
    offsets = [0]
    for object_id, body in objects:
        offsets.append(len(value))
        value.extend(f"{object_id} 0 obj\n".encode() + body + b"\nendobj\n")
    xref_offset = len(value)
    value.extend(b"xref\n0 6\n0000000000 65535 f \n")
    for offset in offsets[1:]:
        value.extend(f"{offset:010d} 00000 n \n".encode())
    value.extend(
        (
            f"trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n"
        ).encode()
    )
    return bytes(value)


def _type1_encrypt(value: bytes) -> bytes:
    result = bytearray()
    state = 55665
    for item in value:
        encrypted = item ^ (state >> 8)
        result.append(encrypted)
        state = ((encrypted + state) * 52845 + 22719) & 0xFFFF
    return bytes(result)


class MultiFormatConformancePdfTests(unittest.TestCase):
    def test_normalization_removes_only_fixed_length_runtime_fields(self) -> None:
        first = (
            b"%PDF-1.7\n"
            b"/CreationDate (D:20260821110000+09'00')\n"
            b"<xmp:MetadataDate>2026-08-21T11:00:00+09:00</xmp:MetadataDate>\n"
            b"/ID [ <0123456789ABCDEF> <0123456789ABCDEF> ]\n"
            b"/DocChecksum /0123456789ABCDEF0123456789ABCDEF\n"
        )
        second = (
            b"%PDF-1.7\n"
            b"/CreationDate (D:20260821120000+09'00')\n"
            b"<xmp:MetadataDate>2026-08-21T12:00:00+09:00</xmp:MetadataDate>\n"
            b"/ID [ <FEDCBA9876543210> <FEDCBA9876543210> ]\n"
            b"/DocChecksum /FEDCBA9876543210FEDCBA9876543210\n"
        )

        normalized_first = normalize_pdf_bytes(first)
        normalized_second = normalize_pdf_bytes(second)

        self.assertEqual(normalized_first, normalized_second)
        self.assertEqual(len(normalized_first), len(first))
        self.assertIn(b"%PDF-1.7", normalized_first)

    def test_each_pdf_stratum_has_an_observable_feature_surface(self) -> None:
        strata = {
            "text-fonts": b"font-variant",
            "vector-transparency": b"<svg",
            "raster-color-space": b"data:image/png;base64,",
            "page-geometry": b"landscape",
            "forms-annotations-links": b"https://example.com/conformance",
            "international": "한글".encode(),
            "mixed-edge": b"mixed-edge-table",
        }

        for ordinal, (stratum, marker) in enumerate(strata.items(), start=1):
            with self.subTest(stratum=stratum):
                value = pdf_case_html(
                    {
                        "id": f"pdf-conformance-{ordinal:03d}",
                        "ordinal": ordinal,
                        "primary_stratum": stratum,
                        "feature_seed": f"{ordinal:064x}",
                    }
                )
                self.assertIn(marker, value)
                self.assertIn(b"pdf-conformance-", value)
                if stratum == "international":
                    self.assertIn(
                        b"font-family:'Apple SD Gothic Neo'",
                        value,
                    )
                    self.assertIn(
                        b"font-family:'Hiragino Sans GB'",
                        value,
                    )
                    self.assertIn(b"font-family:'Amiri'", value)

    def test_pdf_is_rewritten_with_deterministic_link_annotation(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            source_path = Path(temp_dir) / "source.pdf"
            write_positive_source(source_path, "pdf", "link")
            source = source_path.read_bytes()

            first = add_link_annotation(source)
            second = add_link_annotation(source)

        self.assertEqual(first, second)
        self.assertIn(b"/Subtype /Link", first)
        self.assertIn(b"/URI (https://example.com/conformance)", first)
        self.assertNotIn(b"/Prev", first)
        self.assertIn(b"/Size 5 /Root 1 0 R", first)

    def test_xref_stream_is_rewritten_for_structural_validation(self) -> None:
        value = canonicalize_pdf_bytes(_xref_stream_pdf())

        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source = root / "source.pdf"
            source.write_bytes(value)
            validate_source(
                {
                    "id": "xref-stream",
                    "path": source.name,
                    "sha256": sha256_file(source),
                },
                root,
                DocumentFormat.PDF,
                require_valid_format=True,
            )
        self.assertIn(b"xref\n0 6\n", value)
        self.assertNotIn(b"/Type /XRef", value)
        self.assertNotIn(b"/Type /ObjStm", value)
        self.assertIn(b"/CreationDate (0000000000000000000000)", value)
        self.assertNotIn(b"20260821120319", value)

    def test_type1_unique_id_is_canonicalized(self) -> None:
        first = canonicalize_pdf_bytes(_type1_pdf(4_100_001))
        second = canonicalize_pdf_bytes(_type1_pdf(4_100_002))

        self.assertEqual(first, second)
