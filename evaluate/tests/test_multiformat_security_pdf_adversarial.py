from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from evaluate.multiformat_corpus_types import CorpusError, DocumentFormat
from evaluate.multiformat_security_fixture import validate_security_fixture
from evaluate.multiformat_security_source_pdf import _pdf_bytes
from evaluate.multiformat_source_fixture import write_positive_source


class MultiFormatSecurityPdfAdversarialTests(unittest.TestCase):
    def test_malformed_xref_requires_coherent_object_graph(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            source = Path(temp_dir) / "fake.pdf"
            source.write_bytes(
                b"%PDF-1.7\n1 0 obj\n<< /Type /Catalog >>\nendobj\n"
                b"startxref\n0\n%%EOF\n"
            )

            with self.assertRaisesRegex(CorpusError, "security.fixture"):
                validate_security_fixture(
                    source,
                    DocumentFormat.PDF,
                    "malformed-xref",
                )

    def test_page_cycle_is_not_deep_page_tree(self) -> None:
        objects = [
            (1, b"<< /Type /Catalog /Pages 2 0 R >>"),
            (2, b"<< /Type /Pages /Kids [2 0 R] /Count 1 >>"),
        ]
        value, _ = _pdf_bytes(objects, "")
        with tempfile.TemporaryDirectory() as temp_dir:
            source = Path(temp_dir) / "cycle.pdf"
            source.write_bytes(value)

            with self.assertRaisesRegex(CorpusError, "security.fixture"):
                validate_security_fixture(
                    source,
                    DocumentFormat.PDF,
                    "deep-page-tree",
                )

    def test_encrypt_reference_must_be_in_trailer(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            source = Path(temp_dir) / "marker.pdf"
            write_positive_source(source, "pdf", "plain")
            source.write_bytes(source.read_bytes() + b"\n/Encrypt 99 0 R\n")

            with self.assertRaisesRegex(CorpusError, "security.fixture"):
                validate_security_fixture(
                    source,
                    DocumentFormat.PDF,
                    "encrypted-document",
                )

    def test_stream_tokens_are_not_actions(self) -> None:
        objects = [
            (1, b"<< /Type /Catalog /Pages 2 0 R /Metadata 4 0 R >>"),
            (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
            (3, b"<< /Type /Page /Parent 2 0 R >>"),
            (
                4,
                b"<< /Type /Metadata /Length 14 >>\nstream\n/S /JavaScript\nendstream",
            ),
        ]
        value, _ = _pdf_bytes(objects, "")
        with tempfile.TemporaryDirectory() as temp_dir:
            source = Path(temp_dir) / "metadata.pdf"
            source.write_bytes(value)

            with self.assertRaisesRegex(CorpusError, "security.fixture"):
                validate_security_fixture(
                    source,
                    DocumentFormat.PDF,
                    "javascript-action",
                )

    def test_metadata_action_key_has_no_action_semantics(self) -> None:
        objects = [
            (1, b"<< /Type /Catalog /Pages 2 0 R /Metadata 4 0 R >>"),
            (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
            (3, b"<< /Type /Page /Parent 2 0 R >>"),
            (4, b"<< /Type /Metadata /A 5 0 R >>"),
            (5, b"<< /S /JavaScript /JS (app.alert\\(1\\)) >>"),
        ]
        value, _ = _pdf_bytes(objects, "")
        with tempfile.TemporaryDirectory() as temp_dir:
            source = Path(temp_dir) / "metadata-action.pdf"
            source.write_bytes(value)

            with self.assertRaisesRegex(CorpusError, "security.fixture"):
                validate_security_fixture(
                    source,
                    DocumentFormat.PDF,
                    "javascript-action",
                )

    def test_unattached_annotation_does_not_supply_action(self) -> None:
        objects = [
            (1, b"<< /Type /Catalog /Pages 2 0 R /Metadata 4 0 R >>"),
            (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
            (3, b"<< /Type /Page /Parent 2 0 R >>"),
            (4, b"<< /Type /Metadata /Related 5 0 R >>"),
            (5, b"<< /Type /Annot /Subtype /Link /A 6 0 R >>"),
            (6, b"<< /S /JavaScript /JS (app.alert\\(1\\)) >>"),
        ]
        value, _ = _pdf_bytes(objects, "")
        with tempfile.TemporaryDirectory() as temp_dir:
            source = Path(temp_dir) / "unattached-annotation.pdf"
            source.write_bytes(value)

            with self.assertRaisesRegex(CorpusError, "security.fixture"):
                validate_security_fixture(
                    source,
                    DocumentFormat.PDF,
                    "javascript-action",
                )

    def test_indirect_reference_generation_must_match_object(self) -> None:
        objects = [
            (1, b"<< /Type /Catalog /Pages 2 0 R /OpenAction 4 99 R >>"),
            (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
            (3, b"<< /Type /Page /Parent 2 0 R >>"),
            (4, b"<< /S /JavaScript /JS (app.alert\\(1\\)) >>"),
        ]
        value, _ = _pdf_bytes(objects, "")
        with tempfile.TemporaryDirectory() as temp_dir:
            source = Path(temp_dir) / "generation.pdf"
            source.write_bytes(value)

            with self.assertRaisesRegex(CorpusError, "security.fixture"):
                validate_security_fixture(
                    source,
                    DocumentFormat.PDF,
                    "javascript-action",
                )
