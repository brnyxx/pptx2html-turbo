from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_security_fixture import validate_security_fixture
from evaluate.multiformat_security_source import write_security_source
from evaluate.multiformat_source_fixture import SourceFixtureError


class MultiFormatSecuritySourceWriterTests(unittest.TestCase):
    def test_typed_format_writes_semantically_valid_source(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            source = Path(temp_dir) / "external-uri.pdf"

            write_security_source(
                source,
                DocumentFormat.PDF,
                "external-uri",
            )

            validate_security_fixture(
                source,
                DocumentFormat.PDF,
                "external-uri",
            )

    def test_existing_destination_is_never_overwritten(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            source = Path(temp_dir) / "external-uri.pdf"
            source.write_bytes(b"sentinel")

            with self.assertRaises(SourceFixtureError):
                write_security_source(
                    source,
                    DocumentFormat.PDF,
                    "external-uri",
                )

            self.assertEqual(source.read_bytes(), b"sentinel")

    def test_invalid_family_leaves_destination_absent(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            source = Path(temp_dir) / "unknown.pdf"

            with self.assertRaises(SourceFixtureError):
                write_security_source(
                    source,
                    DocumentFormat.PDF,
                    "unknown-family",
                )

            self.assertFalse(source.exists())

    def test_missing_parent_is_reported_as_typed_error(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            source = Path(temp_dir) / "missing" / "external-uri.pdf"

            with self.assertRaises(SourceFixtureError):
                write_security_source(
                    source,
                    DocumentFormat.PDF,
                    "external-uri",
                )

            self.assertFalse(source.exists())


if __name__ == "__main__":
    unittest.main()
