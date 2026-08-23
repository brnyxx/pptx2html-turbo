import json
import tempfile
import unittest
from pathlib import Path

from evaluate.multiformat_evaluator_files import EVALUATOR_FILES
from evaluate.multiformat_evaluator_manifest import validate_evaluator_manifest
from evaluate.multiformat_metric_types import MetricError
from evaluate.scaffold_multiformat_evidence import scaffold_evidence
from evaluate.tests.multiformat_gate_fixture import CONTRACT_PATH, PROJECT_ROOT


class MultiFormatEvaluatorManifestTests(unittest.TestCase):
    def test_manifest_boundary_includes_portable_lock_validation(self) -> None:
        self.assertIn("evaluate/multiformat_reference_profile.py", EVALUATOR_FILES)
        self.assertIn("evaluate/multiformat_portable_lock.py", EVALUATOR_FILES)
        self.assertIn(
            "evaluate/tests/test_multiformat_reference_profile.py",
            EVALUATOR_FILES,
        )
        self.assertIn(
            "evaluate/tests/test_multiformat_portable_lock.py",
            EVALUATOR_FILES,
        )
        for path in (
            "evaluate/jcs.py",
            "evaluate/multiformat_atomic_publish.py",
            "evaluate/multiformat_corpus_identity.py",
            "evaluate/multiformat_corpus_qualification.py",
            "evaluate/multiformat_corpus_admission.py",
            "evaluate/multiformat_corpus_admission_sources.py",
            "evaluate/multiformat_corpus_admission_types.py",
            "evaluate/admit_multiformat_corpus.py",
            "evaluate/tests/test_jcs.py",
            "evaluate/tests/test_admit_multiformat_corpus.py",
            "evaluate/tests/test_multiformat_atomic_publish.py",
            "evaluate/tests/test_multiformat_corpus_identity.py",
            "evaluate/tests/test_multiformat_corpus_admission.py",
            "evaluate/multiformat/reference-routing.v1.json",
            "evaluate/multiformat_reference_routing.py",
            "evaluate/tests/test_multiformat_reference_routing.py",
        ):
            with self.subTest(path=path):
                self.assertIn(path, EVALUATOR_FILES)

    def test_manifest_binds_exact_code_parameters_and_dependencies(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            output = Path(temp_dir) / "wave"
            scaffold_evidence(PROJECT_ROOT, CONTRACT_PATH, output)
            manifest = output / "evidence" / "evaluator-manifest.json"

            digest = validate_evaluator_manifest(
                PROJECT_ROOT,
                CONTRACT_PATH,
                manifest,
            )

            self.assertEqual(len(digest), 64)

    def test_changed_evaluator_digest_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            output = Path(temp_dir) / "wave"
            scaffold_evidence(PROJECT_ROOT, CONTRACT_PATH, output)
            manifest = output / "evidence" / "evaluator-manifest.json"
            value = json.loads(manifest.read_text(encoding="utf-8"))
            value["files"][0]["sha256"] = "0" * 64
            manifest.write_text(json.dumps(value, sort_keys=True), encoding="utf-8")

            with self.assertRaisesRegex(
                MetricError,
                "evaluator.manifest_mismatch",
            ):
                validate_evaluator_manifest(
                    PROJECT_ROOT,
                    CONTRACT_PATH,
                    manifest,
                )

    def test_malformed_manifest_raises_typed_metric_error(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            output = Path(temp_dir) / "wave"
            scaffold_evidence(PROJECT_ROOT, CONTRACT_PATH, output)
            manifest = output / "evidence" / "evaluator-manifest.json"
            value = json.loads(manifest.read_text(encoding="utf-8"))
            del value["algorithm_parameters"]
            manifest.write_text(json.dumps(value, sort_keys=True), encoding="utf-8")

            with self.assertRaisesRegex(
                MetricError,
                "evaluator.manifest_mismatch",
            ):
                validate_evaluator_manifest(
                    PROJECT_ROOT,
                    CONTRACT_PATH,
                    manifest,
                )
