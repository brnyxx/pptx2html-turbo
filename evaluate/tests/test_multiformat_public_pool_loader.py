from __future__ import annotations

import tempfile
import unittest
from dataclasses import FrozenInstanceError, fields
from pathlib import Path

from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_public_pool import load_validated_public_pool_sources
from evaluate.multiformat_public_pool_types import ValidatedPublicPoolSource
from evaluate.tests.multiformat_public_pool_fixture import (
    write_multiformat_public_pool_fixture,
)


class MultiFormatPublicPoolLoaderTests(unittest.TestCase):
    def test_source_type_has_exact_frozen_fields(self) -> None:
        names = tuple(field.name for field in fields(ValidatedPublicPoolSource))
        self.assertEqual(
            names,
            ("document_format", "source_id", "relative_path", "source_sha256"),
        )
        source = ValidatedPublicPoolSource(
            document_format=DocumentFormat.DOCX,
            source_id="source",
            relative_path="source.docx",
            source_sha256="0" * 64,
        )
        with self.assertRaises(FrozenInstanceError):
            source.__setattr__("source_id", "changed")

    def test_loader_returns_tuple(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            fixture = write_multiformat_public_pool_fixture(Path(temp_dir))
            result = load_validated_public_pool_sources(
                fixture.config,
                fixture.manifest,
            )

            self.assertIs(type(result), tuple)
