import json
import tempfile
import unittest
from pathlib import Path

from evaluate.multiformat_candidate_manifest import (
    CandidateManifestError,
    write_candidate_manifests,
)
from evaluate.multiformat_candidate_sources import load_candidate_sources
from evaluate.multiformat_capture_manifest import validate_capture_manifest
from evaluate.multiformat_metric_links import load_metric_spec
from evaluate.multiformat_metric_types import MetricError
from evaluate.multiformat_schema import sha256_file
from evaluate.tests.multiformat_candidate_manifest_fixture import (
    prepare_manifest_runtime,
)
from evaluate.tests.multiformat_candidate_run_fixture import write_candidate_run
from evaluate.tests.multiformat_small_corpus_fixture import ready_fixture


class MultiFormatCandidateManifestTests(unittest.TestCase):
    def test_writes_self_validating_capture_and_independent_determinism_runs(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            contract, corpus = ready_fixture(root)
            source_set = load_candidate_sources(contract, corpus)
            run1 = write_candidate_run(root, source_set, 1)
            run2 = write_candidate_run(root, source_set, 2)
            evaluator = root / "evaluator.json"
            evaluator.write_text("{}", encoding="utf-8")
            runtime = prepare_manifest_runtime(root, contract, corpus, evaluator)

            paths = write_candidate_manifests(
                root,
                root / "candidate",
                source_set,
                run1,
                run2,
                contract,
                corpus,
                evaluator,
                runtime.oracle_lock,
                project_revision="a" * 40,
                runtime_tools=runtime.tools,
                runtime_artifacts=runtime.artifacts,
                receipt_signer=runtime.artifacts["receipt_signer_binary"],
                font_bundle_sha256=runtime.font_bundle_sha256,
            )

            validated = validate_capture_manifest(
                paths.capture,
                "candidate",
                load_metric_spec(corpus),
                sha256_file(contract),
                sha256_file(corpus),
                sha256_file(evaluator),
                sha256_file(runtime.oracle_lock),
                "a" * 40,
                root,
                runtime.oracle_lock,
            )
            determinism = json.loads(paths.determinism.read_text(encoding="utf-8"))
            run1_inventory = determinism["runs"][0]["files"][0]["inventory"]["path"]
            run2_inventory = determinism["runs"][1]["files"][0]["inventory"]["path"]
            self.assertNotEqual(run1_inventory, run2_inventory)
            self.assertEqual(len(validated.units), 7)
            self.assertEqual(len(validated.files), 6)

    def test_refuses_to_publish_when_clean_runs_differ(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            contract, corpus = ready_fixture(root)
            source_set = load_candidate_sources(contract, corpus)
            run1 = write_candidate_run(root, source_set, 1)
            run2 = write_candidate_run(root, source_set, 2)
            run2.sources[0].units[0].png.write_bytes(b"different")
            evaluator = root / "evaluator.json"
            evaluator.write_text("{}", encoding="utf-8")
            runtime = prepare_manifest_runtime(root, contract, corpus, evaluator)

            with self.assertRaisesRegex(
                CandidateManifestError,
                "determinism",
            ):
                write_candidate_manifests(
                    root,
                    root / "candidate",
                    source_set,
                    run1,
                    run2,
                    contract,
                    corpus,
                    evaluator,
                    runtime.oracle_lock,
                    project_revision="a" * 40,
                    runtime_tools=runtime.tools,
                    runtime_artifacts=runtime.artifacts,
                    receipt_signer=runtime.artifacts["receipt_signer_binary"],
                    font_bundle_sha256=runtime.font_bundle_sha256,
                )

    def test_product_validation_rejects_a_replayed_or_edited_receipt(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            contract, corpus = ready_fixture(root)
            source_set = load_candidate_sources(contract, corpus)
            run1 = write_candidate_run(root, source_set, 1)
            run2 = write_candidate_run(root, source_set, 2)
            evaluator = root / "evaluator.json"
            evaluator.write_text("{}", encoding="utf-8")
            runtime = prepare_manifest_runtime(root, contract, corpus, evaluator)
            paths = write_candidate_manifests(
                root,
                root / "candidate",
                source_set,
                run1,
                run2,
                contract,
                corpus,
                evaluator,
                runtime.oracle_lock,
                project_revision="a" * 40,
                runtime_tools=runtime.tools,
                runtime_artifacts=runtime.artifacts,
                receipt_signer=runtime.artifacts["receipt_signer_binary"],
                font_bundle_sha256=runtime.font_bundle_sha256,
            )
            capture = json.loads(paths.capture.read_text(encoding="utf-8"))
            receipt = root / capture["execution_receipt"]["path"]
            receipt_value = json.loads(receipt.read_text(encoding="utf-8"))
            receipt_value["run_nonce"] = "f" * 64
            receipt.write_text(json.dumps(receipt_value), encoding="utf-8")

            with self.assertRaises(MetricError):
                validate_capture_manifest(
                    paths.capture,
                    "candidate",
                    load_metric_spec(corpus),
                    sha256_file(contract),
                    sha256_file(corpus),
                    sha256_file(evaluator),
                    sha256_file(runtime.oracle_lock),
                    "a" * 40,
                    root,
                    runtime.oracle_lock,
                )
