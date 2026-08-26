from __future__ import annotations

import tempfile
import unittest
import zlib
from pathlib import Path

from evaluate import multiformat_conformance_pdf as pdf
from evaluate.multiformat_conformance_pdf_cases import pdf_case_html
from evaluate.multiformat_corpus_sources import validate_source
from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_pdf_link_annotation import add_link_annotation
from evaluate.multiformat_reference_routing import load_reference_routing
from evaluate.multiformat_schema import sha256_file
from evaluate.multiformat_source_fixture import write_positive_source

ROOT = Path(__file__).resolve().parents[2]
ROUTING_TABLE = ROOT / "evaluate/multiformat/reference-routing.v1.json"


def _xref_stream_pdf(
    *,
    escaped_object_fields: bool = False,
    object_stream_extra: bytes = b"",
) -> bytes:
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
    object_fields = (
        b"/Ty#70e /Obj#53tm /Len#67th 6 0 R /#4E 1 /Fi#72st 4 /Fil#74er /Flate#44ecode"
        if escaped_object_fields
        else b"/Type /ObjStm /Length 6 0 R /N 1 /First 4 /Filter /FlateDecode"
    )
    value.extend(
        b"4 0 obj\n<< " + object_fields + b" " + object_stream_extra + b" >>\nstream\n"
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


def _type1_pdf(
    unique_id: int,
    *,
    escaped_fields: bool = False,
    font_extra: bytes = b"",
) -> bytes:
    clear = b"%!FontType1-1.0: Deterministic 1.0\neexec\n"
    encrypted = _type1_encrypt(f"    /UniqueID {unique_id} def\n".encode())
    trailer = b"\ncleartomark\n"
    program = clear + encrypted + trailer
    compressed = zlib.compress(program)
    catalog = (
        b"<< /Ty#70e /Cat#61log /Pages 2 0 R >>"
        if escaped_fields
        else b"<< /Type /Catalog /Pages 2 0 R >>"
    )
    font_fields = (
        f"/Len#67th 5 0 R /Fil#74er /Flate#44ecode /Len#67th1 {len(clear)} "
        f"/L#65ngth2 {len(encrypted)} /Le#6Egth3 {len(trailer)}"
        if escaped_fields
        else f"/Length 5 0 R /Filter /FlateDecode /Length1 {len(clear)} "
        f"/Length2 {len(encrypted)} /Length3 {len(trailer)}"
    )
    objects = [
        (1, catalog),
        (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (
            3,
            (
                b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] "
                b"/Contents 6 0 R >>"
            ),
        ),
        (
            4,
            (f"<< {font_fields} ".encode() + font_extra + b" >>\nstream\n")
            + compressed
            + b"\nendstream",
        ),
        (5, str(len(compressed)).encode()),
        (
            6,
            b"<< /Length 34 >>\nstream\nBT (Portable evidence text) Tj ET\nendstream",
        ),
    ]
    value = bytearray(b"%PDF-1.5\n")
    offsets = [0]
    for object_id, body in objects:
        offsets.append(len(value))
        value.extend(f"{object_id} 0 obj\n".encode() + body + b"\nendobj\n")
    xref_offset = len(value)
    value.extend(b"xref\n0 7\n0000000000 65535 f \n")
    for offset in offsets[1:]:
        value.extend(f"{offset:010d} 00000 n \n".encode())
    value.extend(
        (
            f"trailer\n<< /Size 7 /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n"
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
                    fonts = (b"Apple SD Gothic Neo", b"Hiragino Sans GB", b"Amiri")
                    for font in fonts:
                        self.assertIn(font, value)

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
        value = pdf.canonicalize_pdf_bytes(_xref_stream_pdf())

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
        # Canonical numbering is dense: the dropped ObjStm, its length, and
        # the XRef object leave no free slot behind.
        self.assertIn(b"xref\n0 5\n", value)
        self.assertNotIn(b"0000000000 00000 f \n", value)
        self.assertNotIn(b"/Type /XRef", value)
        self.assertNotIn(b"/Type /ObjStm", value)
        self.assertIn(b"/CreationDate (D:20260821120319+09'00)", value)

    def test_type1_unique_id_is_canonicalized(self) -> None:
        first = pdf.canonicalize_pdf_bytes(_type1_pdf(4_100_001))
        second = pdf.canonicalize_pdf_bytes(_type1_pdf(4_100_002))

        self.assertEqual(first, second)

    def test_boundary_is_idempotent_valid_and_text_preserving(self) -> None:
        source = _type1_pdf(4_100_001)

        first = pdf.canonicalize_pdf_bytes(source)
        second = pdf.canonicalize_pdf_bytes(source)
        repeated = pdf.canonicalize_pdf_bytes(first)

        self.assertEqual(first, second)
        self.assertEqual(first, repeated)
        self.assertIn(b"Portable evidence text", first)

    def test_unsupported_or_ambiguous_constructs_fail_with_typed_error(self) -> None:
        source = _type1_pdf(4_100_001)
        trailer = b"/Root 1 0 R"
        attacks = (
            ("encryption", source.replace(trailer, trailer + b" /Encrypt 8 0 R")),
            ("incremental-update", source.replace(trailer, trailer + b" /Prev 12")),
            ("hybrid-xref", source.replace(trailer, trailer + b" /XRefStm 12")),
        )

        for construct, value in attacks:
            with (
                self.subTest(construct=construct),
                self.assertRaises(pdf.PdfUnsupportedConstructError) as raised,
            ):
                pdf.canonicalize_pdf_bytes(value)
            self.assertEqual(raised.exception.construct, construct)

    def test_identity_binds_version_implementation_and_reference_route(self) -> None:
        identity = pdf.pdf_canonicalizer_identity()
        routing = load_reference_routing(ROUTING_TABLE)
        changed_sources = tuple(
            (name, value + (b"\n# changed rule" if index == 0 else b""))
            for index, (name, value) in enumerate(pdf.canonicalizer_sources())
        )

        self.assertEqual(identity.version, "2")
        self.assertRegex(identity.implementation_sha256, r"^[0-9a-f]{64}$")
        self.assertEqual(routing.canonicalizer_version, identity.version)
        self.assertEqual(
            routing.canonicalizer_implementation_sha256,
            identity.implementation_sha256,
        )
        self.assertNotEqual(
            pdf.canonicalizer_implementation_sha256(changed_sources),
            identity.implementation_sha256,
        )

    def test_fixed_ids_keep_source_and_output_identity_separate(self) -> None:
        source = _type1_pdf(4_100_001)
        first = source.replace(b"/Root 1 0 R", b"/Root 1 0 R /ID [<AAAA><BBBB>]")
        second = source.replace(b"/Root 1 0 R", b"/Root 1 0 R /ID [<CCCC><DDDD>]")
        canonical = pdf.canonicalize_pdf_bytes(first)

        self.assertNotEqual(first, canonical)
        self.assertEqual(canonical, pdf.canonicalize_pdf_bytes(second))
        self.assertIn(b"/ID [<0000><0000>]", canonical)
        self.assertIn(b"Portable evidence text", canonical)
