import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from evaluate.multiformat_evaluator_files import EVALUATOR_FILES
from evaluate.multiformat_evaluator_manifest import validate_evaluator_manifest
from evaluate.multiformat_evaluator_portable_wave_files import (
    PORTABLE_WAVE_ENGINE_FILES,
    PORTABLE_WAVE_TEST_FILES,
)
from evaluate.multiformat_metric_types import MetricError
from evaluate.scaffold_multiformat_evidence import scaffold_evidence
from evaluate.tests.multiformat_gate_fixture import CONTRACT_PATH, PROJECT_ROOT


class MultiFormatEvaluatorManifestTests(unittest.TestCase):
    def test_manifest_binds_portable_wave_producers_and_tests(self) -> None:
        for path in (
            "evaluate/materialize_multiformat_candidate_runtime_locks.py",
            "evaluate/materialize_multiformat_candidate_runtime_locks_cli.py",
        ):
            self.assertIn(path, PORTABLE_WAVE_ENGINE_FILES)
        self.assertIn(
            "evaluate/tests/test_materialize_multiformat_candidate_runtime_locks.py",
            PORTABLE_WAVE_TEST_FILES,
        )
        for path in (*PORTABLE_WAVE_ENGINE_FILES, *PORTABLE_WAVE_TEST_FILES):
            with self.subTest(path=path):
                self.assertIn(path, EVALUATOR_FILES)

    def test_manifest_binds_exact_portable_receipt_boundary(self) -> None:
        expected = {
            "evaluate/multiformat_portable_receipt.py",
            "evaluate/multiformat_portable_receipt_identity.py",
            "evaluate/multiformat_portable_receipt_replay.py",
            "evaluate/multiformat_portable_receipt_context.py",
            "evaluate/multiformat_portable_receipt_trust.py",
            "evaluate/multiformat_portable_receipt_validation.py",
            "evaluate/multiformat_portable_receipt_sources.py",
            "evaluate/multiformat_candidate_portable_receipt.py",
            "evaluate/materialize_multiformat_portable_receipt_wrapper.py",
            "evaluate/multiformat_portable_receipt_executor.py",
            "evaluate/tests/multiformat_portable_receipt_fixture.py",
            "evaluate/tests/test_multiformat_candidate_portable_receipt.py",
            "evaluate/tests/test_multiformat_portable_wave_signers.py",
            "evaluate/tests/test_multiformat_portable_receipt.py",
            "evaluate/tests/test_multiformat_portable_receipt_trust_flow.py",
        }
        actual = {
            path
            for path in EVALUATOR_FILES
            if "portable_receipt" in path
            or path.endswith("test_multiformat_portable_wave_signers.py")
        }

        self.assertEqual(actual, expected)

    def test_portable_outer_trust_modules_and_regressions_are_digest_bound(
        self,
    ) -> None:
        for path in (
            "evaluate/multiformat_portable_outer_sandbox.py",
            "evaluate/multiformat_portable_package_inventory.py",
            "evaluate/multiformat_portable_reference_process.py",
            "evaluate/multiformat_portable_trust_artifacts.py",
        ):
            self.assertIn(path, PORTABLE_WAVE_ENGINE_FILES)
            self.assertIn(path, EVALUATOR_FILES)
        for path in (
            "evaluate/tests/test_materialize_multiformat_portable_locks.py",
            "evaluate/tests/test_multiformat_portable_lock.py",
            "evaluate/tests/test_multiformat_portable_receipt_trust_flow.py",
            "evaluate/tests/test_multiformat_portable_reference_runner.py",
        ):
            self.assertIn(path, PORTABLE_WAVE_TEST_FILES)
            self.assertIn(path, EVALUATOR_FILES)

    def test_manifest_binds_candidate_process_isolation_and_regressions(self) -> None:
        production = (
            "evaluate/create_multiformat_oracle_sentinel.py",
            "evaluate/multiformat_candidate_attestation_signing_io.py",
            "evaluate/multiformat_candidate_sandbox.py",
        )
        regression = "evaluate/tests/test_multiformat_candidate_process_isolation.py"
        for path in production:
            self.assertIn(path, PORTABLE_WAVE_ENGINE_FILES)
            self.assertIn(path, EVALUATOR_FILES)
        self.assertIn(regression, PORTABLE_WAVE_TEST_FILES)
        self.assertIn(regression, EVALUATOR_FILES)

    def test_manifest_boundary_includes_portable_lock_validation(self) -> None:
        self.assertIn("evaluate/multiformat_reference_profile.py", EVALUATOR_FILES)
        self.assertIn("evaluate/multiformat_portable_lock.py", EVALUATOR_FILES)
        for path in (
            "evaluate/generate_multiformat_legacy_conformance.py",
            "evaluate/multiformat_legacy_conformance.py",
            "evaluate/multiformat_legacy_process.py",
            "evaluate/multiformat_legacy_runtime.py",
            "evaluate/multiformat_legacy_sources.py",
            "evaluate/multiformat_legacy_types.py",
            "evaluate/tests/test_generate_multiformat_legacy_conformance.py",
            "evaluate/tests/test_generate_multiformat_legacy_conformance_cli.py",
            "evaluate/tests/test_multiformat_legacy_runtime.py",
        ):
            self.assertIn(path, EVALUATOR_FILES)
        for path in (
            "evaluate/collect_multiformat_legacy_binary_pool.py",
            "evaluate/multiformat/legacy-binary-sources.v1.json",
            "evaluate/multiformat_legacy_binary_config.py",
            "evaluate/multiformat_legacy_binary_pool.py",
            "evaluate/multiformat_legacy_binary_validation.py",
            "evaluate/tests/test_collect_multiformat_legacy_binary_pool.py",
            "evaluate/tests/test_collect_multiformat_legacy_binary_pool_cli.py",
        ):
            self.assertIn(path, EVALUATOR_FILES)
        for path in (
            "evaluate/generate_multiformat_security_sources.py",
            "evaluate/validate_multiformat_security_sources.py",
            "evaluate/multiformat_security_snapshot_cli.py",
            "evaluate/tests/test_multiformat_security_sources_cli.py",
        ):
            self.assertIn(path, EVALUATOR_FILES)
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
            "evaluate/multiformat_capture_profile.py",
            "evaluate/multiformat_capture_upstream.py",
            "evaluate/multiformat_portable_capture.py",
            "evaluate/multiformat_portable_receipt.py",
            "evaluate/multiformat_portable_receipt_identity.py",
            "evaluate/multiformat_portable_receipt_replay.py",
            "evaluate/multiformat_portable_receipt_context.py",
            "evaluate/multiformat_portable_receipt_trust.py",
            "evaluate/multiformat_portable_receipt_validation.py",
            "evaluate/tests/multiformat_portable_capture_fixture.py",
            "evaluate/tests/multiformat_portable_receipt_fixture.py",
            "evaluate/tests/test_multiformat_portable_receipt.py",
            "evaluate/tests/test_multiformat_portable_receipt_trust_flow.py",
        ):
            with self.subTest(path=path):
                self.assertIn(path, EVALUATOR_FILES)
        for path in (
            "evaluate/assemble_multiformat_ready_corpora.py",
            "evaluate/validate_multiformat_ready_corpora.py",
            "evaluate/multiformat_ready_assembly.py",
            "evaluate/multiformat_ready_validation.py",
            "evaluate/capture_multiformat_native_units.py",
            "evaluate/multiformat_native_unit_capture.py",
            "evaluate/tests/test_multiformat_ready_cli.py",
            "evaluate/tests/test_multiformat_ready_assembly.py",
            "evaluate/tests/test_capture_multiformat_native_units.py",
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

    def test_cryptography_dependency_version_mismatch_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            output = Path(temp_dir) / "wave"
            scaffold_evidence(PROJECT_ROOT, CONTRACT_PATH, output)
            manifest = output / "evidence" / "evaluator-manifest.json"
            dependencies = json.loads(manifest.read_text(encoding="utf-8"))[
                "dependencies"
            ]

            with (
                mock.patch(
                    "evaluate.multiformat_evaluator_manifest.importlib.metadata.version",
                    side_effect=lambda name: (
                        "0.0.0" if name == "cryptography" else dependencies[name]
                    ),
                ),
                self.assertRaisesRegex(MetricError, "cryptography"),
            ):
                validate_evaluator_manifest(PROJECT_ROOT, CONTRACT_PATH, manifest)

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
