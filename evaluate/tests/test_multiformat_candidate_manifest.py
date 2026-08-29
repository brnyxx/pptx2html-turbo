import json
import tempfile
import unittest
from pathlib import Path

from PIL import Image

from evaluate.multiformat_candidate_determinism import (
    _inventory_equivalent,
    _visually_equivalent,
)
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
    def test_native_determinism_allows_only_bounded_rendering_jitter(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            left_inventory = root / "left.json"
            right_inventory = root / "right.json"
            left_inventory.write_text(
                json.dumps(
                    {
                        "objects": [
                            {
                                "box": [10.0, 20.0, 30.0, 40.0],
                                "baseline": 42.0,
                                "value": "stable",
                            }
                        ]
                    }
                ),
                encoding="utf-8",
            )
            right_inventory.write_text(
                json.dumps(
                    {
                        "objects": [
                            {
                                "box": [12.0, 18.0, 30.0, 40.0],
                                "baseline": 40.0,
                                "value": "stable",
                            }
                        ]
                    }
                ),
                encoding="utf-8",
            )
            left_png = root / "left.png"
            right_png = root / "right.png"
            left = Image.new("RGB", (256, 256), "white")
            right = Image.new("RGB", (256, 256), "white")
            for image, offset in ((left, 0), (right, 1)):
                for x in range(80, 176):
                    for y in range(80 + offset, 176 + offset):
                        image.putpixel((x, y), (0, 0, 0))
            left.save(left_png)
            right.save(right_png)

            self.assertTrue(_inventory_equivalent(left_inventory, right_inventory))
            self.assertTrue(_visually_equivalent(left_png, right_png))

            right_value = json.loads(right_inventory.read_text(encoding="utf-8"))
            right_value["objects"][0]["box"][1] = 71.0
            right_inventory.write_text(json.dumps(right_value), encoding="utf-8")
            self.assertFalse(_inventory_equivalent(left_inventory, right_inventory))

            right_value["objects"][0]["box"][1] = 18.0
            right_value["objects"][0]["value"] = "changed"
            right_inventory.write_text(json.dumps(right_value), encoding="utf-8")
            self.assertFalse(_inventory_equivalent(left_inventory, right_inventory))

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
