from __future__ import annotations

import tempfile
import unittest
from dataclasses import FrozenInstanceError, fields
from pathlib import Path

from evaluate.multiformat_candidate_artifacts import write_canonical_json
from evaluate.multiformat_corpus_items import object_list
from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_schema import object_value, string_value
from evaluate.multiformat_public_pool import load_validated_public_pool_sources
from evaluate.multiformat_public_pool_types import ValidatedPublicPoolSource
from evaluate.tests.multiformat_public_pool_fixture import (
    write_multiformat_public_pool_fixture,
)
from evaluate.multiformat_strict_json import read_strict_object


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

    def test_loader_sorts_shuffled_source_records(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            fixture = write_multiformat_public_pool_fixture(Path(temp_dir))
            values = read_strict_object(fixture.manifest)
            formats = object_value(values, "formats")
            expected: list[tuple[str, str]] = []
            for format_name, format_value in formats.items():
                if not isinstance(format_value, dict):
                    raise AssertionError("format must be an object")
                sources = format_value.get("sources")
                if not isinstance(sources, list):
                    raise AssertionError("sources must be an array")
                sources.reverse()
                expected.extend(
                    (format_name, string_value(source, "id"))
                    for source in object_list(format_value, "sources", "test")
                )
            expected.sort()
            write_canonical_json(fixture.manifest, values)

            result = load_validated_public_pool_sources(
                fixture.config,
                fixture.manifest,
            )

            self.assertEqual(
                [(item.document_format.value, item.source_id) for item in result],
                expected,
            )
