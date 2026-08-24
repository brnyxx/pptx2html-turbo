from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from evaluate.multiformat_corpus_types import CorpusError, DocumentFormat
from evaluate.multiformat_security_fixture import validate_security_fixture
from evaluate.multiformat_security_source_pdf import _pdf_bytes


class MultiFormatSecurityPdfContainerAdversarialTests(unittest.TestCase):
    def _assert_rejected(
        self,
        objects: list[tuple[int, bytes]],
        family: str,
    ) -> None:
        value, _ = _pdf_bytes(objects, "")
        with tempfile.TemporaryDirectory() as temp_dir:
            source = Path(temp_dir) / "adversarial.pdf"
            source.write_bytes(value)
            with self.assertRaisesRegex(CorpusError, "security.fixture"):
                validate_security_fixture(source, DocumentFormat.PDF, family)

    def test_nested_annots_key_does_not_attach_annotation(self) -> None:
        self._assert_rejected(
            [
                (1, b"<< /Type /Catalog /Pages 2 0 R >>"),
                (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
                (
                    3,
                    b"<< /Type /Page /Parent 2 0 R /Metadata << /Annots [4 0 R] >> >>",
                ),
                (4, b"<< /Type /Annot /Subtype /Link /A 5 0 R >>"),
                (5, b"<< /S /JavaScript /JS (app.alert\\(1\\)) >>"),
            ],
            "javascript-action",
        )

    def test_nested_annotation_reference_is_not_direct_array_item(self) -> None:
        self._assert_rejected(
            [
                (1, b"<< /Type /Catalog /Pages 2 0 R >>"),
                (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
                (3, b"<< /Type /Page /Parent 2 0 R /Annots [<< /X 4 0 R >>] >>"),
                (4, b"<< /Type /Annot /Subtype /Link /A 5 0 R >>"),
                (5, b"<< /S /JavaScript /JS (app.alert\\(1\\)) >>"),
            ],
            "javascript-action",
        )

    def test_embedded_files_must_be_under_catalog_names(self) -> None:
        self._assert_rejected(
            [
                (
                    1,
                    b"<< /Type /Catalog /Pages 2 0 R "
                    b"/EmbeddedFiles << /Names [(payload) 4 0 R] >> >>",
                ),
                (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
                (3, b"<< /Type /Page /Parent 2 0 R >>"),
                (4, b"<< /Type /Filespec /F (payload) /EF << /F 5 0 R >> >>"),
                (5, b"<< /Type /EmbeddedFile /Length 1 >>\nstream\nx\nendstream"),
            ],
            "embedded-file",
        )

    def test_embedded_files_outside_empty_names_are_ignored(self) -> None:
        self._assert_rejected(
            [
                (
                    1,
                    b"<< /Type /Catalog /Pages 2 0 R /Names << >> "
                    b"/EmbeddedFiles << /Names [(payload) 4 0 R] >> >>",
                ),
                (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
                (3, b"<< /Type /Page /Parent 2 0 R >>"),
                (4, b"<< /Type /Filespec /F (payload) /EF << /F 5 0 R >> >>"),
                (5, b"<< /Type /EmbeddedFile /Length 1 >>\nstream\nx\nendstream"),
            ],
            "embedded-file",
        )

    def test_nested_filespec_reference_is_not_direct_names_item(self) -> None:
        self._assert_rejected(
            [
                (
                    1,
                    b"<< /Type /Catalog /Pages 2 0 R "
                    b"/Names << /EmbeddedFiles << "
                    b"/Names [(payload) << /X 4 0 R >>] >> >> >>",
                ),
                (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
                (3, b"<< /Type /Page /Parent 2 0 R >>"),
                (4, b"<< /Type /Filespec /F (payload) /EF << /F 5 0 R >> >>"),
                (5, b"<< /Type /EmbeddedFile /Length 1 >>\nstream\nx\nendstream"),
            ],
            "embedded-file",
        )

    def test_embedded_names_require_name_filespec_pairs(self) -> None:
        self._assert_rejected(
            [
                (
                    1,
                    b"<< /Type /Catalog /Pages 2 0 R "
                    b"/Names << /EmbeddedFiles << /Names [4 0 R] >> >> >>",
                ),
                (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
                (3, b"<< /Type /Page /Parent 2 0 R >>"),
                (4, b"<< /Type /Filespec /F (payload) /EF << /F 5 0 R >> >>"),
                (5, b"<< /Type /EmbeddedFile /Length 1 >>\nstream\nx\nendstream"),
            ],
            "embedded-file",
        )

    def test_embedded_hex_names_must_contain_only_hex_digits(self) -> None:
        for invalid_name in (b"<ZZ>", b"<0G>"):
            with self.subTest(invalid_name=invalid_name):
                self._assert_rejected(
                    [
                        (
                            1,
                            b"<< /Type /Catalog /Pages 2 0 R "
                            b"/Names << /EmbeddedFiles << /Names ["
                            + invalid_name
                            + b" 4 0 R] >> >> >>",
                        ),
                        (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
                        (3, b"<< /Type /Page /Parent 2 0 R >>"),
                        (
                            4,
                            b"<< /Type /Filespec /F (payload) /EF << /F 5 0 R >> >>",
                        ),
                        (
                            5,
                            b"<< /Type /EmbeddedFile /Length 1 >>"
                            b"\nstream\nx\nendstream",
                        ),
                    ],
                    "embedded-file",
                )

    def test_metadata_image_is_not_a_page_resource(self) -> None:
        self._assert_rejected(
            [
                (1, b"<< /Type /Catalog /Pages 2 0 R /Metadata 4 0 R >>"),
                (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
                (3, b"<< /Type /Page /Parent 2 0 R >>"),
                (
                    4,
                    b"<< /Type /XObject /Subtype /Image /Width 200000 "
                    b"/Height 200000 /Length 1 >>\nstream\nx\nendstream",
                ),
            ],
            "oversized-image",
        )

    def test_xobject_outside_empty_resources_is_ignored(self) -> None:
        self._assert_rejected(
            [
                (1, b"<< /Type /Catalog /Pages 2 0 R >>"),
                (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
                (
                    3,
                    b"<< /Type /Page /Parent 2 0 R /Resources << >> "
                    b"/XObject << /Im0 4 0 R >> >>",
                ),
                (
                    4,
                    b"<< /Type /XObject /Subtype /Image /Width 200000 "
                    b"/Height 200000 /Length 1 >>\nstream\nx\nendstream",
                ),
            ],
            "oversized-image",
        )

    def test_nested_image_reference_is_not_direct_xobject_value(self) -> None:
        self._assert_rejected(
            [
                (1, b"<< /Type /Catalog /Pages 2 0 R >>"),
                (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
                (
                    3,
                    b"<< /Type /Page /Parent 2 0 R "
                    b"/Resources << /XObject << /Group << /X 4 0 R >> >> >> >>",
                ),
                (
                    4,
                    b"<< /Type /XObject /Subtype /Image /Width 200000 "
                    b"/Height 200000 /Length 1 >>\nstream\nx\nendstream",
                ),
            ],
            "oversized-image",
        )
