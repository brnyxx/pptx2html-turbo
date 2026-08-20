import json
import tempfile
import unittest
from pathlib import Path

from evaluate.multiformat_corpus import (
    CorpusError,
    CorpusStatus,
    validate_corpus_manifest,
)
from evaluate.scaffold_multiformat_evidence import scaffold_evidence
from evaluate.tests.multiformat_small_corpus_fixture import ready_fixture

PROJECT_ROOT = Path(__file__).resolve().parents[2]
CONTRACT_PATH = PROJECT_ROOT / "evaluate" / "multiformat" / "contract.v1.json"


class MultiFormatCorpusTests(unittest.TestCase):
    def test_ready_manifest_enforces_exact_tracks_and_sources(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            contract, manifest = ready_fixture(root)

            result = validate_corpus_manifest(contract, manifest)

            self.assertEqual(result.status, CorpusStatus.READY)
            self.assertEqual(result.conformance_units, 2)
            self.assertEqual(result.blind_files, 5)
            self.assertEqual(result.security_cases, 2)
            self.assertEqual(result.blind_producers, 5)

    def test_incomplete_scaffold_stays_incomplete(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            output = Path(temp_dir) / "wave"
            scaffold_evidence(PROJECT_ROOT, CONTRACT_PATH, output)

            contract = json.loads(CONTRACT_PATH.read_text(encoding="utf-8"))
            for document_format in contract["required_formats"]:
                with self.subTest(document_format=document_format):
                    result = validate_corpus_manifest(
                        CONTRACT_PATH,
                        output / "corpora" / document_format / "manifest.json",
                    )
                    self.assertEqual(result.status, CorpusStatus.INCOMPLETE)
                    self.assertEqual(result.conformance_units, 0)
                    self.assertEqual(result.blind_files, 0)
                    self.assertEqual(result.security_cases, 0)

    def test_duplicate_blind_source_or_template_is_rejected(self) -> None:
        for option, reason in [
            ({"duplicate_blind_hash": True}, "blind.sha256"),
            ({"duplicate_template": True}, "blind.template_family"),
        ]:
            with self.subTest(reason=reason):
                with tempfile.TemporaryDirectory() as temp_dir:
                    root = Path(temp_dir)
                    contract, manifest_path = ready_fixture(root, **option)

                    with self.assertRaisesRegex(CorpusError, reason):
                        validate_corpus_manifest(contract, manifest_path)

    def test_blind_track_requires_five_independent_producers(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            contract, manifest = ready_fixture(root, few_producers=True)

            with self.assertRaisesRegex(CorpusError, "blind.producers"):
                validate_corpus_manifest(contract, manifest)

    def test_source_path_and_hash_are_bound(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            contract, traversal = ready_fixture(root, traversal=True)
            with self.assertRaisesRegex(CorpusError, "source.path"):
                validate_corpus_manifest(contract, traversal)

        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            contract, bad_hash = ready_fixture(root, bad_hash=True)
            with self.assertRaisesRegex(CorpusError, "source.sha256"):
                validate_corpus_manifest(contract, bad_hash)

    def test_positive_sources_must_match_the_declared_format(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            contract, manifest = ready_fixture(root, invalid_signature=True)

            with self.assertRaisesRegex(CorpusError, "source.format"):
                validate_corpus_manifest(contract, manifest)

    def test_primary_stratum_quotas_are_exact(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            contract, manifest = ready_fixture(root, wrong_quota=True)

            with self.assertRaisesRegex(CorpusError, "conformance.stratum"):
                validate_corpus_manifest(contract, manifest)
