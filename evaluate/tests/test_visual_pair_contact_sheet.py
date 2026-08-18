import tempfile
import unittest
from pathlib import Path

from PIL import Image

from evaluate.visual_pair_contact_sheet import build_contact_sheets


class VisualPairContactSheetTests(unittest.TestCase):
    def test_builds_labeled_sheet_for_every_matched_slide_pair(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            references = root / "references" / "deck"
            candidates = root / "candidates" / "deck"
            references.mkdir(parents=True)
            candidates.mkdir(parents=True)
            for index in range(3):
                Image.new("RGB", (960, 540), (20 * index, 0, 0)).save(
                    references / f"slide_{index}.png"
                )
                Image.new("RGB", (960, 540), (0, 20 * index, 0)).save(
                    candidates / f"slide_{index}.png"
                )

            sheets = build_contact_sheets(
                root / "references",
                root / "candidates",
                root / "sheets",
                thumbnail_width=240,
                thumbnail_height=135,
                pairs_per_row=2,
            )

            self.assertEqual([path.name for path in sheets], ["deck.png"])
            with Image.open(sheets[0]) as sheet:
                self.assertEqual(sheet.size, (960, 306))

    def test_rejects_missing_candidate_pair(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            reference = root / "references" / "deck"
            reference.mkdir(parents=True)
            Image.new("RGB", (960, 540), "white").save(
                reference / "slide_0.png"
            )

            with self.assertRaisesRegex(
                FileNotFoundError,
                "CANDIDATE_SLIDE_MISSING",
            ):
                build_contact_sheets(
                    root / "references",
                    root / "candidates",
                    root / "sheets",
                )

    def test_resizes_same_aspect_libreoffice_reference(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            reference = root / "references" / "deck"
            candidate = root / "candidates" / "deck"
            reference.mkdir(parents=True)
            candidate.mkdir(parents=True)
            Image.new("RGB", (2001, 1125), "white").save(
                reference / "slide_0.png"
            )
            Image.new("RGB", (960, 540), "white").save(
                candidate / "slide_0.png"
            )

            sheets = build_contact_sheets(
                root / "references",
                root / "candidates",
                root / "sheets",
            )

            self.assertEqual(len(sheets), 1)

    def test_letterboxes_matching_4x3_pairs_without_distortion(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            reference = root / "references" / "deck"
            candidate = root / "candidates" / "deck"
            reference.mkdir(parents=True)
            candidate.mkdir(parents=True)
            Image.new("RGB", (2000, 1500), "red").save(
                reference / "slide_0.png"
            )
            Image.new("RGB", (960, 720), "red").save(
                candidate / "slide_0.png"
            )

            sheets = build_contact_sheets(
                root / "references",
                root / "candidates",
                root / "sheets",
            )

            with Image.open(sheets[0]) as sheet:
                self.assertEqual(sheet.getpixel((5, 50)), (255, 255, 255))
                self.assertEqual(sheet.getpixel((40, 50)), (255, 0, 0))


if __name__ == "__main__":
    unittest.main()
