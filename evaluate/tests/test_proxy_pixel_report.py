import tempfile
import unittest
from pathlib import Path

import numpy as np
from PIL import Image

from evaluate.proxy_pixel_report import compare_pair, create_report


def _save_rgb(path: Path, pixels: np.ndarray) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    Image.fromarray(pixels.astype(np.uint8), mode="RGB").save(path)


class ProxyPixelReportTests(unittest.TestCase):
    def test_compare_pair_reports_exact_pixel_metrics(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            candidate = np.zeros((2, 2, 3), dtype=np.uint8)
            reference = candidate.copy()
            reference[0, 0, 2] = 255
            candidate_path = root / "candidate.png"
            reference_path = root / "reference.png"
            _save_rgb(candidate_path, candidate)
            _save_rgb(reference_path, reference)

            result = compare_pair(candidate_path, reference_path)

            self.assertEqual(result["mismatched_pixels"], 1)
            self.assertEqual(result["mismatched_pixel_ratio"], 0.25)
            self.assertAlmostEqual(result["mae"], 255 / 12)
            self.assertEqual(result["max_channel_delta"], 255)
            self.assertAlmostEqual(result["similarity"], 91.6666666667)

    def test_compare_pair_resizes_only_same_aspect_reference(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            candidate_path = root / "candidate.png"
            reference_path = root / "reference.png"
            _save_rgb(candidate_path, np.zeros((2, 4, 3), dtype=np.uint8))
            _save_rgb(reference_path, np.zeros((4, 8, 3), dtype=np.uint8))

            with self.assertRaisesRegex(ValueError, "dimensions"):
                compare_pair(candidate_path, reference_path)

            result = compare_pair(
                candidate_path,
                reference_path,
                allow_reference_resize=True,
            )
            self.assertEqual(result["similarity"], 100.0)
            self.assertTrue(result["reference_resized"])

    def test_create_report_matches_relative_slide_paths(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            candidates = root / "candidates"
            references = root / "references"
            pixels = np.full((2, 2, 3), 127, dtype=np.uint8)
            _save_rgb(candidates / "deck_a" / "slide_0.png", pixels)
            _save_rgb(candidates / "deck_b" / "slide_1.png", pixels)
            _save_rgb(references / "deck_a" / "slide_0.png", pixels)
            _save_rgb(references / "deck_b" / "slide_1.png", pixels)

            report = create_report(candidates, references)

            self.assertEqual(report["slide_count"], 2)
            self.assertEqual(report["corpus_similarity"], 100.0)
            self.assertTrue(report["all_slides_meet_95_percent"])
