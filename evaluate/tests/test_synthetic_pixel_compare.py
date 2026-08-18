import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

from PIL import Image

from evaluate import synthetic_pixel_compare


class SyntheticPixelCompareTests(unittest.TestCase):
    def test_synthetic_comparator_module_exists(self) -> None:
        module = importlib.util.find_spec("evaluate.synthetic_pixel_compare")

        self.assertIsNotNone(module)

    def test_exact_directory_pair_reports_zero_mismatches(self) -> None:
        # Given
        compare = getattr(
            synthetic_pixel_compare,
            "compare_synthetic_batch",
            None,
        )

        # When/Then
        self.assertTrue(callable(compare))
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            references = root / "references"
            candidates = root / "candidates"
            _write_image(references / "synthetic_01" / "slide_0.png", "112233")
            _write_image(references / "synthetic_01" / "slide_1.png", "AABBCC")
            _write_image(candidates / "synthetic_01" / "slide_0.png", "112233")
            _write_image(candidates / "synthetic_01" / "slide_1.png", "AABBCC")

            report = compare(references, candidates)

            self.assertTrue(report["ok"])
            self.assertEqual(report["comparison"], "exact-rgba")
            self.assertEqual(report["reference_oracle"], "synthetic-scene-spec")
            self.assertEqual(report["deck_count"], 1)
            self.assertEqual(report["slide_count"], 2)
            self.assertEqual(report["mismatched_pixels"], 0)
            self.assertEqual(report["max_channel_delta"], 0)

    def test_single_channel_change_fails_and_writes_diff(self) -> None:
        # Given
        compare = getattr(
            synthetic_pixel_compare,
            "compare_synthetic_batch",
            None,
        )

        # When/Then
        self.assertTrue(callable(compare))
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            references = root / "references"
            candidates = root / "candidates"
            diffs = root / "diffs"
            reference = references / "synthetic_01" / "slide_0.png"
            candidate = candidates / "synthetic_01" / "slide_0.png"
            _write_image(reference, "112233")
            _write_image(candidate, "112233")
            with Image.open(candidate) as source:
                changed = source.convert("RGBA")
            changed.putpixel((1, 2), (255, 34, 51, 255))
            changed.save(candidate)

            report = compare(references, candidates, diff_dir=diffs)

            self.assertFalse(report["ok"])
            self.assertEqual(report["mismatched_pixels"], 1)
            self.assertEqual(report["max_channel_delta"], 238)
            self.assertTrue((diffs / "synthetic_01" / "Slide1.PNG").is_file())

    def test_missing_candidate_is_rejected(self) -> None:
        # Given
        compare = getattr(
            synthetic_pixel_compare,
            "compare_synthetic_batch",
            None,
        )
        error_type = getattr(
            synthetic_pixel_compare,
            "SyntheticPixelError",
            None,
        )

        # When/Then
        self.assertTrue(callable(compare))
        self.assertIsInstance(error_type, type)
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            references = root / "references"
            candidates = root / "candidates"
            _write_image(references / "synthetic_01" / "slide_0.png", "112233")
            candidates.mkdir()

            with self.assertRaisesRegex(error_type, "SYNTHETIC_CANDIDATE_MISSING"):
                compare(references, candidates)

    def test_missing_candidate_root_is_rejected_at_boundary(self) -> None:
        # Given
        compare = synthetic_pixel_compare.compare_synthetic_batch

        # When/Then
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            references = root / "references"
            _write_image(references / "synthetic_01" / "slide_0.png", "112233")

            with self.assertRaisesRegex(
                synthetic_pixel_compare.SyntheticPixelError,
                "SYNTHETIC_CANDIDATE_ROOT_INVALID",
            ):
                compare(references, root / "missing-candidates")

    def test_cli_writes_machine_readable_exact_report(self) -> None:
        # Given
        main = getattr(synthetic_pixel_compare, "main", None)

        # When/Then
        self.assertTrue(callable(main))
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            references = root / "references"
            candidates = root / "candidates"
            output = root / "report.json"
            _write_image(references / "synthetic_01" / "slide_0.png", "112233")
            _write_image(candidates / "synthetic_01" / "slide_0.png", "112233")

            exit_code = main(
                [
                    "--reference-dir",
                    str(references),
                    "--candidate-dir",
                    str(candidates),
                    "--width",
                    "4",
                    "--height",
                    "3",
                    "--output-json",
                    str(output),
                    "--expected-decks",
                    "1",
                    "--expected-slides",
                    "1",
                    "--width",
                    "4",
                    "--height",
                    "3",
                ]
            )

            self.assertEqual(exit_code, 0)
            self.assertTrue(json.loads(output.read_text(encoding="utf-8"))["ok"])

    def test_cli_default_contract_rejects_partial_exact_batch(self) -> None:
        # Given
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            references = root / "references"
            candidates = root / "candidates"
            output = root / "report.json"
            _write_image(references / "synthetic_01" / "slide_0.png", "112233")
            _write_image(candidates / "synthetic_01" / "slide_0.png", "112233")

            # When
            exit_code = synthetic_pixel_compare.main(
                [
                    "--reference-dir",
                    str(references),
                    "--candidate-dir",
                    str(candidates),
                    "--width",
                    "4",
                    "--height",
                    "3",
                    "--output-json",
                    str(output),
                ]
            )

            # Then
            report = json.loads(output.read_text(encoding="utf-8"))
            self.assertEqual(exit_code, 1)
            self.assertIn("SYNTHETIC_DECK_COUNT_MISMATCH", report["error"])

    def test_expected_resolution_is_enforced_for_every_slide(self) -> None:
        # Given
        compare = synthetic_pixel_compare.compare_synthetic_batch

        # When/Then
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            references = root / "references"
            candidates = root / "candidates"
            for output in (references, candidates):
                _write_image(
                    output / "synthetic_01" / "slide_0.png",
                    "112233",
                )
                _write_image(
                    output / "synthetic_01" / "slide_1.png",
                    "445566",
                    size=(5, 3),
                )

            with self.assertRaisesRegex(
                synthetic_pixel_compare.SyntheticPixelError,
                "SYNTHETIC_RESOLUTION_MISMATCH:synthetic_01:slide_1.png",
            ):
                compare(
                    references,
                    candidates,
                    expected_deck_count=1,
                    expected_slide_count=2,
                    expected_resolution=(4, 3),
                )


def _write_image(
    path: Path,
    color: str,
    *,
    size: tuple[int, int] = (4, 3),
) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    Image.new("RGBA", size, f"#{color}").save(path)


if __name__ == "__main__":
    unittest.main()
