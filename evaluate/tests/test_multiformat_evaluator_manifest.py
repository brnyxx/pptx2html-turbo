import json
import tempfile
import unittest
from pathlib import Path

from evaluate.multiformat_evaluator_manifest import validate_evaluator_manifest
from evaluate.multiformat_metric_types import MetricError
from evaluate.scaffold_multiformat_evidence import scaffold_evidence
from evaluate.tests.multiformat_gate_fixture import CONTRACT_PATH, PROJECT_ROOT


class MultiFormatEvaluatorManifestTests(unittest.TestCase):
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
