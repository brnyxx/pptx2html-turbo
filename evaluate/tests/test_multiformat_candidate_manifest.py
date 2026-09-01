import json
import tempfile
import unittest
from pathlib import Path

from PIL import Image

from evaluate.multiformat_candidate_determinism import (
    CandidateDeterminismError,
    _inventory_equivalent,
    _visually_equivalent,
    validate_clean_runs,
)
from evaluate.multiformat_candidate_manifest import (
    CandidateManifestError,
    write_candidate_manifests,
)
from evaluate.multiformat_candidate_sources import (
    CandidateSource,
    CandidateSourceSet,
    CandidateUnitSpec,
    load_candidate_sources,
)
from evaluate.multiformat_capture_manifest import validate_capture_manifest
from evaluate.multiformat_corpus_types import DocumentFormat
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

    def test_native_inventory_bounds_text_fragmentation_and_loss(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            left = root / "left.json"
            right = root / "right.json"
            stable = "Aspose.Words支持DOC，DOCX，RTF，HTML，OpenDocument，PDF，XPS，EPUB等格式。"
            left.write_text(
                json.dumps({"texts": [{"value": stable}], "objects": []}),
                encoding="utf-8",
            )
            right.write_text(
                json.dumps(
                    {
                        "texts": [
                            {"value": stable[:18]},
                            {"value": " " + stable[18:36]},
                            {"value": stable[36:]},
                        ],
                        "objects": [],
                    }
                ),
                encoding="utf-8",
            )
            self.assertTrue(_inventory_equivalent(left, right))

            right.write_text(
                json.dumps(
                    {
                        "texts": [{"value": stable[:-4]}],
                        "objects": [],
                    }
                ),
                encoding="utf-8",
            )
            self.assertFalse(_inventory_equivalent(left, right))

    def test_pptx_determinism_allows_only_visual_png_variation(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source_set = CandidateSourceSet(
                DocumentFormat.PPTX,
                (
                    CandidateSource(
                        "blind",
                        "blind-pptx-apache-poi-004",
                        "a" * 64,
                        root / "source.pptx",
                        (CandidateUnitSpec("blind-pptx-apache-poi-004-unit-3", 3),),
                    ),
                ),
            )
            run1 = write_candidate_run(root, source_set, 1)
            run2 = write_candidate_run(root, source_set, 2)
            left_unit = run1.sources[0].units[0]
            right_unit = run2.sources[0].units[0]

            Image.new("RGB", (192, 192), (10, 20, 30)).save(left_unit.png)
            Image.new("RGB", (192, 192), (10, 20, 30)).save(right_unit.png)
            with Image.open(right_unit.png) as image:
                image.save(right_unit.png, compress_level=0)
            self.assertNotEqual(
                sha256_file(left_unit.png),
                sha256_file(right_unit.png),
            )
            validate_clean_runs(source_set, run1, run2)

            Image.new("RGB", (192, 192), (90, 80, 70)).save(right_unit.png)
            with self.assertRaises(CandidateDeterminismError):
                validate_clean_runs(source_set, run1, run2)

            Image.new("RGB", (192, 192), (10, 20, 30)).save(right_unit.png)
            run2.sources[0].html.write_text("<html>changed</html>", encoding="utf-8")
            with self.assertRaises(CandidateDeterminismError):
                validate_clean_runs(source_set, run1, run2)

            run2.sources[0].html.write_text(
                run1.sources[0].html.read_text(encoding="utf-8"),
                encoding="utf-8",
            )
            right_unit.inventory.write_text('{"changed":true}', encoding="utf-8")
            with self.assertRaises(CandidateDeterminismError):
                validate_clean_runs(source_set, run1, run2)

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
