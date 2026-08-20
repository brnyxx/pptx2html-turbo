import json
import tempfile
import unittest
from pathlib import Path

from evaluate.multiformat_candidate_sources import load_candidate_sources
from evaluate.multiformat_corpus_types import CorpusError, DocumentFormat
from evaluate.tests.multiformat_small_corpus_fixture import ready_fixture


class MultiFormatCandidateSourceTests(unittest.TestCase):
    def test_loads_every_positive_source_and_frozen_unit_identity(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            contract, manifest = ready_fixture(Path(temp_dir))

            result = load_candidate_sources(contract, manifest)

            self.assertEqual(result.document_format, DocumentFormat.DOCX)
            self.assertEqual(len(result.sources), 6)
            self.assertEqual(
                [unit.unit_id for unit in result.sources[0].units],
                ["unit-1", "unit-2"],
            )
            self.assertEqual(result.sources[1].track, "blind")
            self.assertEqual(result.sources[1].units[0].ordinal, 1)
            self.assertTrue(result.sources[1].units[0].unit_id.endswith("-unit-1"))

    def test_rejects_source_or_unit_ids_that_can_escape_artifact_paths(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            contract, manifest = ready_fixture(Path(temp_dir))
            value = json.loads(manifest.read_text(encoding="utf-8"))
            value["tracks"]["conformance"]["items"][0]["units"][0]["id"] = "../escape"
            manifest.write_text(json.dumps(value, sort_keys=True), encoding="utf-8")

            with self.assertRaisesRegex(CorpusError, "conformance.unit.id"):
                load_candidate_sources(contract, manifest)
