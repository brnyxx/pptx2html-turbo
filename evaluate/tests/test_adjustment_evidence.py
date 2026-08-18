import importlib.util
import json
import tempfile
import unittest
from pathlib import Path
from zipfile import ZIP_DEFLATED, ZipFile

from evaluate import adjustment_evidence, adjustment_visual_evidence
from evaluate.create_exhaustive_adjustment_deck import (
    write_exhaustive_adjustment_deck,
)

ROOT = Path(__file__).resolve().parents[2]
CANONICAL = ROOT / "evaluate" / "preset_adjustments.json"


class AdjustmentEvidenceTests(unittest.TestCase):
    temporary: tempfile.TemporaryDirectory[str]
    deck: Path
    manifest: Path

    @classmethod
    def setUpClass(cls) -> None:
        cls.temporary = tempfile.TemporaryDirectory()
        cls.deck, cls.manifest = write_exhaustive_adjustment_deck(
            CANONICAL,
            Path(cls.temporary.name) / "corpus",
        )

    @classmethod
    def tearDownClass(cls) -> None:
        cls.temporary.cleanup()

    def test_adjustment_evidence_module_exists(self) -> None:
        module = importlib.util.find_spec("evaluate.adjustment_evidence")

        self.assertIsNotNone(module)

    def test_corpus_validator_proves_all_manifest_and_ooxml_cases(self) -> None:
        report = adjustment_evidence.validate_adjustment_corpus(
            CANONICAL,
            self.manifest,
            self.deck,
        )

        self.assertTrue(report["ok"])
        self.assertEqual(report["adjustment_pair_count"], 300)
        self.assertEqual(report["case_count"], 900)
        self.assertEqual(report["slide_count"], 75)
        self.assertEqual(report["ooxml_shapes_verified"], 900)

    def test_corpus_validator_rejects_missing_case(self) -> None:
        payload = json.loads(self.manifest.read_text(encoding="utf-8"))
        payload["entries"].pop()
        mutated = Path(self.temporary.name) / "missing-case.json"
        mutated.write_text(json.dumps(payload), encoding="utf-8")

        with self.assertRaisesRegex(
            adjustment_evidence.AdjustmentEvidenceError,
            "ADJUSTMENT_CASE_COUNT_MISMATCH",
        ):
            adjustment_evidence.validate_adjustment_corpus(
                CANONICAL,
                mutated,
                self.deck,
            )

    def test_corpus_validator_rejects_ooxml_assignment_mismatch(self) -> None:
        mutated = Path(self.temporary.name) / "wrong-value.pptx"
        with ZipFile(self.deck) as source, ZipFile(
            mutated,
            "w",
            ZIP_DEFLATED,
        ) as target:
            for member in source.infolist():
                payload = source.read(member)
                if member.filename == "ppt/slides/slide1.xml":
                    payload = payload.replace(
                        b'fmla="val 0"',
                        b'fmla="val 1"',
                        1,
                    )
                target.writestr(member, payload)

        with self.assertRaisesRegex(
            adjustment_evidence.AdjustmentEvidenceError,
            "ADJUSTMENT_OOXML_MISMATCH:ADJ_000_LOW",
        ):
            adjustment_evidence.validate_adjustment_corpus(
                CANONICAL,
                self.manifest,
                mutated,
            )

    def test_visual_validator_links_all_slides_and_shapes(self) -> None:
        proxy, shapes = self._write_visual_reports()

        report = adjustment_visual_evidence.validate_adjustment_visual_evidence(
            self.manifest,
            proxy,
            shapes,
        )

        self.assertTrue(report["ok"])
        self.assertEqual(report["slide_count"], 75)
        self.assertEqual(report["shape_count"], 900)
        self.assertEqual(report["adjustment_pair_count"], 300)

    def test_visual_validator_rejects_duplicate_slide_pair(self) -> None:
        proxy, shapes = self._write_visual_reports()
        payload = json.loads(proxy.read_text(encoding="utf-8"))
        payload["slides"][-1]["candidate"] = payload["slides"][-2]["candidate"]
        proxy.write_text(json.dumps(payload), encoding="utf-8")

        with self.assertRaisesRegex(
            adjustment_visual_evidence.AdjustmentVisualEvidenceError,
            "ADJUSTMENT_PROXY_SLIDE_DUPLICATE:slide_73",
        ):
            adjustment_visual_evidence.validate_adjustment_visual_evidence(
                self.manifest,
                proxy,
                shapes,
            )

    def test_visual_validator_rejects_missing_shape_pair(self) -> None:
        proxy, shapes = self._write_visual_reports()
        payload = json.loads(shapes.read_text(encoding="utf-8"))
        payload["all_shapes"].pop()
        shapes.write_text(json.dumps(payload), encoding="utf-8")

        with self.assertRaisesRegex(
            adjustment_visual_evidence.AdjustmentVisualEvidenceError,
            "ADJUSTMENT_SHAPE_EVIDENCE_MISSING:ADJ_299_HIGH",
        ):
            adjustment_visual_evidence.validate_adjustment_visual_evidence(
                self.manifest,
                proxy,
                shapes,
            )

    def test_visual_validator_rejects_slide_below_threshold(self) -> None:
        proxy, shapes = self._write_visual_reports()
        payload = json.loads(proxy.read_text(encoding="utf-8"))
        payload["slides"][0]["similarity"] = 94.99
        proxy.write_text(json.dumps(payload), encoding="utf-8")

        with self.assertRaisesRegex(
            adjustment_visual_evidence.AdjustmentVisualEvidenceError,
            "ADJUSTMENT_PROXY_BELOW_THRESHOLD:slide_0",
        ):
            adjustment_visual_evidence.validate_adjustment_visual_evidence(
                self.manifest,
                proxy,
                shapes,
            )

    def test_visual_validator_rejects_shape_below_ssim_threshold(self) -> None:
        proxy, shapes = self._write_visual_reports()
        payload = json.loads(shapes.read_text(encoding="utf-8"))
        payload["all_shapes"][0]["fg_ssim"] = 0.749
        shapes.write_text(json.dumps(payload), encoding="utf-8")

        with self.assertRaisesRegex(
            adjustment_visual_evidence.AdjustmentVisualEvidenceError,
            "ADJUSTMENT_SHAPE_BELOW_THRESHOLD:ADJ_000_LOW",
        ):
            adjustment_visual_evidence.validate_adjustment_visual_evidence(
                self.manifest,
                proxy,
                shapes,
            )

    def _write_visual_reports(self) -> tuple[Path, Path]:
        corpus = json.loads(self.manifest.read_text(encoding="utf-8"))
        proxy = Path(self.temporary.name) / "proxy.json"
        shapes = Path(self.temporary.name) / "shapes.json"
        proxy.write_text(
            json.dumps(
                {
                    "slide_count": 75,
                    "all_slides_meet_95_percent": True,
                    "slides": [
                        {
                            "candidate": (
                                f"/tmp/candidates/all_adjustments/"
                                f"slide_{slide_index}.png"
                            ),
                            "similarity": 97.0,
                        }
                        for slide_index in range(75)
                    ],
                }
            ),
            encoding="utf-8",
        )
        shapes.write_text(
            json.dumps(
                {
                    "summary": {"shape_count": 900},
                    "all_shapes": [
                        {
                            "shape_name": entry["shape_name"],
                            "ref_cov12": 80.0,
                            "ref_cov24": 85.0,
                            "cand_cov12": 82.0,
                            "cand_cov24": 87.0,
                            "fg_ssim": 0.95,
                            "mask_iou": 0.9,
                        }
                        for entry in corpus["entries"]
                    ],
                }
            ),
            encoding="utf-8",
        )
        return proxy, shapes


if __name__ == "__main__":
    unittest.main()
