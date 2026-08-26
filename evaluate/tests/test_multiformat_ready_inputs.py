from __future__ import annotations

import tempfile
import unittest
from collections import Counter
from pathlib import Path
from typing import ClassVar
from unittest import mock

from evaluate.multiformat_candidate_artifacts import write_canonical_json
from evaluate.multiformat_corpus_items import object_list
from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_ready_inputs import (
    _before_final_snapshot_identity,
    load_ready_inputs,
)
from evaluate.multiformat_ready_types import (
    ReadyBlind,
    ReadyConformance,
    ReadyInputError,
    ReadyInputFailure,
    ReadySecurity,
)
from evaluate.multiformat_schema import object_value, string_value
from evaluate.multiformat_strict_json import read_strict_object
from evaluate.tests.multiformat_ready_fixture import (
    ReadyInputFixture,
    make_ready_input_fixture,
)


class MultiFormatReadyInputTests(unittest.TestCase):
    fixture_root: ClassVar[tempfile.TemporaryDirectory[str]]
    fixture: ClassVar[ReadyInputFixture]

    @classmethod
    def setUpClass(cls) -> None:
        cls.fixture_root = tempfile.TemporaryDirectory()
        cls.fixture = make_ready_input_fixture(Path(cls.fixture_root.name))

    @classmethod
    def tearDownClass(cls) -> None:
        cls.fixture_root.cleanup()

    def test_snapshot_race_seam_is_a_deterministic_noop(self) -> None:
        self.assertIsNone(_before_final_snapshot_identity("fixture", None))

    def test_loads_exact_typed_frozen_source_inventory(self) -> None:
        result = load_ready_inputs(self.fixture.paths)
        self.assertEqual(len(result.sources), 1295)
        self.assertEqual(len(result.supports), 180)
        counts = Counter(
            (
                source.document_format,
                type(source.details),
            )
            for source in result.sources
        )
        for document_format in DocumentFormat:
            self.assertEqual(counts[document_format, ReadyConformance], 100)
            self.assertEqual(counts[document_format, ReadyBlind], 75)
            self.assertEqual(counts[document_format, ReadySecurity], 10)
        self.assertEqual(
            list(result.sources),
            sorted(
                result.sources,
                key=lambda item: (
                    item.document_format.value,
                    {ReadyConformance: 0, ReadyBlind: 1, ReadySecurity: 2}[
                        type(item.details)
                    ],
                    item.source_id,
                ),
            ),
        )

    def test_plan_joins_provenance_and_inventory_counts_are_exercised(self) -> None:
        result = load_ready_inputs(self.fixture.paths)
        binary = next(
            source
            for source in result.sources
            if isinstance(source.details, ReadyConformance)
            and source.details.provenance is not None
        )
        self.assertIsInstance(binary.details, ReadyConformance)
        assert isinstance(binary.details, ReadyConformance)
        self.assertEqual(binary.details.primary_stratum, "binary-specific")
        self.assertIsNotNone(binary.details.provenance)
        assert binary.details.provenance is not None
        self.assertTrue(binary.details.provenance.independently_authored)
        blind = next(
            source
            for source in result.sources
            if isinstance(source.details, ReadyBlind)
        )
        assert isinstance(blind.details, ReadyBlind)
        self.assertEqual(blind.unit_count, 1)
        self.assertEqual(blind.details.background, "light")
        self.assertTrue(blind.source_path.is_file())

    def test_supports_are_owner_prefixed_and_bound_to_modern_cases(self) -> None:
        result = load_ready_inputs(self.fixture.paths)
        primary_ids = {
            (item.document_format, item.source_id) for item in result.sources
        }
        for support in result.supports:
            self.assertEqual(
                support.support_id,
                f"{support.owner_format.value}-support-{support.modern_case_id}",
            )
            self.assertEqual(
                support.filename,
                f"{support.support_id}.{support.support_format.value}",
            )
            self.assertIn((support.support_format, support.modern_case_id), primary_ids)
            self.assertNotIn((support.support_format, support.support_id), primary_ids)

    def test_modern_status_mutation_fails_typed(self) -> None:
        manifest = self.fixture.paths.pptx_conformance
        original = manifest.read_bytes()
        try:
            values = read_strict_object(manifest)
            values["status"] = "DRAFT"
            write_canonical_json(manifest, values)
            with self.assertRaises(ReadyInputError) as raised:
                load_ready_inputs(self.fixture.paths)
            self.assertEqual(
                raised.exception.failure, ReadyInputFailure.CONFORMANCE_INVALID
            )
        finally:
            manifest.write_bytes(original)

    def test_source_mutation_at_final_identity_seam_is_detected_without_sleep(
        self,
    ) -> None:
        manifest = self.fixture.paths.public_pool_manifest
        values = object_value(read_strict_object(manifest), "formats")
        first_format = min(values)
        format_value = object_value(values, first_format)
        relative = string_value(
            object_list(format_value, "sources", "ready.test")[0], "path"
        )
        source = manifest.parent / relative
        original = source.read_bytes()
        mutated = False

        def race(label: str, root: Path | None) -> None:
            nonlocal mutated
            if label == "public-pool" and not mutated:
                mutated = True
                source.write_bytes(original + b"changed")

        try:
            with (
                mock.patch(
                    "evaluate.multiformat_ready_inputs._before_final_snapshot_identity",
                    side_effect=race,
                ),
                self.assertRaises(ReadyInputError) as raised,
            ):
                load_ready_inputs(self.fixture.paths)
            self.assertEqual(raised.exception.failure, ReadyInputFailure.SOURCE_CHANGED)
        finally:
            source.write_bytes(original)

    def test_identical_byte_inode_substitution_is_detected(self) -> None:
        manifest = self.fixture.paths.security_manifest
        replacement = manifest.with_suffix(".replacement")
        original = manifest.with_suffix(".original")
        replaced = False

        def race(label: str, root: Path | None) -> None:
            nonlocal replaced
            if label == "security" and not replaced:
                replaced = True
                replacement.write_bytes(manifest.read_bytes())
                manifest.rename(original)
                replacement.rename(manifest)

        try:
            with (
                mock.patch(
                    "evaluate.multiformat_ready_inputs._before_final_snapshot_identity",
                    side_effect=race,
                ),
                self.assertRaises(ReadyInputError) as raised,
            ):
                load_ready_inputs(self.fixture.paths)
            self.assertEqual(raised.exception.failure, ReadyInputFailure.SOURCE_CHANGED)
        finally:
            if replaced:
                manifest.unlink()
                original.rename(manifest)

    def test_identical_source_inode_substitution_is_detected(self) -> None:
        manifest = self.fixture.paths.public_pool_manifest
        formats = object_value(read_strict_object(manifest), "formats")
        format_value = object_value(formats, min(formats))
        relative = string_value(
            object_list(format_value, "sources", "ready.test")[0], "path"
        )
        source = manifest.parent / relative
        replacement = Path(self.fixture_root.name) / "source-replacement"
        original = Path(self.fixture_root.name) / "source-original"
        replaced = False

        def race(label: str, root: Path | None) -> None:
            nonlocal replaced
            if label == "public-pool" and not replaced:
                replaced = True
                replacement.write_bytes(source.read_bytes())
                source.rename(original)
                replacement.rename(source)

        try:
            with (
                mock.patch(
                    "evaluate.multiformat_ready_inputs._before_final_snapshot_identity",
                    side_effect=race,
                ),
                self.assertRaises(ReadyInputError) as raised,
            ):
                load_ready_inputs(self.fixture.paths)
            self.assertEqual(raised.exception.failure, ReadyInputFailure.SOURCE_CHANGED)
        finally:
            if replaced:
                source.unlink()
                original.rename(source)


if __name__ == "__main__":
    unittest.main()
