import importlib.util
import json
import tempfile
import unittest
from pathlib import Path
from zipfile import ZipFile

from evaluate import visual_element_evidence


class VisualElementEvidenceTests(unittest.TestCase):
    def test_element_evidence_validator_module_exists(self) -> None:
        module = importlib.util.find_spec("evaluate.visual_element_evidence")

        self.assertIsNotNone(module)

    def test_validator_links_every_challenge_element_to_proxy_slides(self) -> None:
        # Given
        validate = getattr(
            visual_element_evidence,
            "validate_visual_element_evidence",
            None,
        )

        # When/Then
        self.assertTrue(callable(validate))
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            manifest = root / "manifest.json"
            report = root / "report.json"
            decks = root / "decks"
            _write_manifest(manifest)
            _write_proxy_report(report)
            _write_deck(decks)

            result = validate(
                manifest,
                report,
                challenge_deck_root=decks,
                expected_deck_count=1,
                expected_slide_count=10,
            )

            self.assertTrue(result["ok"])
            self.assertEqual(result["element_count"], 2)
            self.assertEqual(result["minimum_similarity"], 97.0)

    def test_validator_rejects_element_slide_below_proxy_threshold(self) -> None:
        # Given
        validate = getattr(
            visual_element_evidence,
            "validate_visual_element_evidence",
            None,
        )
        error_type = getattr(
            visual_element_evidence,
            "VisualElementEvidenceError",
            None,
        )

        # When/Then
        self.assertTrue(callable(validate))
        self.assertIsInstance(error_type, type)
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            manifest = root / "manifest.json"
            report = root / "report.json"
            decks = root / "decks"
            _write_manifest(manifest)
            _write_proxy_report(report, arrow_similarity=94.99)
            _write_deck(decks)

            with self.assertRaisesRegex(
                error_type,
                "VISUAL_ELEMENT_PROXY_BELOW_THRESHOLD:arrow",
            ):
                validate(
                    manifest,
                    report,
                    challenge_deck_root=decks,
                    expected_deck_count=1,
                    expected_slide_count=10,
                )

    def test_validator_rejects_missing_deck_element_pair(self) -> None:
        # Given
        validate = getattr(
            visual_element_evidence,
            "validate_visual_element_evidence",
            None,
        )
        error_type = getattr(
            visual_element_evidence,
            "VisualElementEvidenceError",
            None,
        )

        # When/Then
        self.assertTrue(callable(validate))
        self.assertIsInstance(error_type, type)
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            manifest = root / "manifest.json"
            report = root / "report.json"
            decks = root / "decks"
            _write_manifest(manifest)
            _write_proxy_report(report, declared_slide_count=20)
            _write_deck(decks)

            with self.assertRaisesRegex(
                error_type,
                "VISUAL_ELEMENT_PROXY_BATCH_MISSING:challenge_02:slide_0",
            ):
                validate(
                    manifest,
                    report,
                    challenge_deck_root=decks,
                    expected_deck_count=2,
                    expected_slide_count=20,
                )

    def test_validator_rejects_duplicate_proxy_slide_keys(self) -> None:
        # Given
        validate = visual_element_evidence.validate_visual_element_evidence
        error_type = visual_element_evidence.VisualElementEvidenceError

        # When/Then
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            manifest = root / "manifest.json"
            report = root / "report.json"
            decks = root / "decks"
            _write_manifest(manifest)
            _write_proxy_report(report, duplicate_last_slide=True)
            _write_deck(decks)

            with self.assertRaisesRegex(
                error_type,
                "VISUAL_ELEMENT_PROXY_DUPLICATE:challenge_01:slide_8",
            ):
                validate(
                    manifest,
                    report,
                    challenge_deck_root=decks,
                    expected_deck_count=1,
                    expected_slide_count=10,
                )

    def test_validator_rejects_missing_element_marker_in_deck(self) -> None:
        # Given
        validate = visual_element_evidence.validate_visual_element_evidence
        error_type = visual_element_evidence.VisualElementEvidenceError

        # When/Then
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            manifest = root / "manifest.json"
            report = root / "report.json"
            decks = root / "decks"
            _write_manifest(manifest)
            _write_proxy_report(report)
            _write_deck(decks, table_payload="")

            with self.assertRaisesRegex(
                error_type,
                "VISUAL_ELEMENT_OOXML_MISSING:table:challenge_01",
            ):
                validate(
                    manifest,
                    report,
                    challenge_deck_root=decks,
                    expected_deck_count=1,
                    expected_slide_count=10,
                )

    def test_validator_rejects_empty_element_marker(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            manifest = root / "manifest.json"
            report = root / "report.json"
            decks = root / "decks"
            _write_manifest(manifest, arrow_marker="")
            _write_proxy_report(report)
            _write_deck(decks)

            with self.assertRaisesRegex(
                visual_element_evidence.VisualElementEvidenceError,
                "VISUAL_ELEMENT_MARKER_INVALID:arrow",
            ):
                visual_element_evidence.validate_visual_element_evidence(
                    manifest,
                    report,
                    challenge_deck_root=decks,
                    expected_deck_count=1,
                    expected_slide_count=10,
                )

    def test_validator_rejects_marker_on_wrong_slide(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            manifest = root / "manifest.json"
            report = root / "report.json"
            decks = root / "decks"
            _write_manifest(manifest)
            _write_proxy_report(report)
            _write_deck(decks, arrow_part="ppt/slides/slide1.xml")

            with self.assertRaisesRegex(
                visual_element_evidence.VisualElementEvidenceError,
                "VISUAL_ELEMENT_OOXML_MISSING:arrow:challenge_01",
            ):
                visual_element_evidence.validate_visual_element_evidence(
                    manifest,
                    report,
                    challenge_deck_root=decks,
                    expected_deck_count=1,
                    expected_slide_count=10,
                )

    def test_validator_rejects_corrupt_deck_archive(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            manifest = root / "manifest.json"
            report = root / "report.json"
            decks = root / "decks"
            _write_manifest(manifest)
            _write_proxy_report(report)
            decks.mkdir()
            _ = (decks / "challenge_01.pptx").write_bytes(b"not-a-zip")

            with self.assertRaisesRegex(
                visual_element_evidence.VisualElementEvidenceError,
                "VISUAL_ELEMENT_DECK_INVALID",
            ):
                visual_element_evidence.validate_visual_element_evidence(
                    manifest,
                    report,
                    challenge_deck_root=decks,
                    expected_deck_count=1,
                    expected_slide_count=10,
                )

    def test_validator_rejects_non_finite_similarity(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            manifest = root / "manifest.json"
            report = root / "report.json"
            decks = root / "decks"
            _write_manifest(manifest)
            _write_proxy_report(report, arrow_similarity=float("nan"))
            _write_deck(decks)

            with self.assertRaisesRegex(
                visual_element_evidence.VisualElementEvidenceError,
                "VISUAL_ELEMENT_FIELD_INVALID:slide.similarity",
            ):
                visual_element_evidence.validate_visual_element_evidence(
                    manifest,
                    report,
                    challenge_deck_root=decks,
                    expected_deck_count=1,
                    expected_slide_count=10,
                )


def _write_manifest(
    path: Path,
    *,
    arrow_marker: str = 'prst="rightArrow"',
) -> None:
    _write_json(
        path,
        {
            "elements": [
                {
                    "id": "arrow",
                    "evidence": [
                        {
                            "tier": "challenge-proxy",
                            "source": "challenge:slide_2:stress_deck:_shapes:rightArrow",
                            "ooxml_marker": arrow_marker,
                            "ooxml_part": "ppt/slides/slide3.xml",
                        }
                    ],
                },
                {
                    "id": "table",
                    "evidence": [
                        {
                            "tier": "challenge-proxy",
                            "source": "challenge:slide_4:table",
                            "ooxml_marker": "<a:tbl>",
                            "ooxml_part": "ppt/slides/slide5.xml",
                        }
                    ],
                },
            ]
        },
    )


def _write_proxy_report(
    path: Path,
    *,
    arrow_similarity: float = 97.0,
    declared_slide_count: int = 10,
    duplicate_last_slide: bool = False,
) -> None:
    slides = []
    for slide_index in range(10):
        similarity = arrow_similarity if slide_index == 2 else 98.0
        slides.append(
            {
                "candidate": (
                    f"/tmp/candidates/challenge_01/slide_{slide_index}.png"
                ),
                "similarity": similarity,
            }
        )
    if duplicate_last_slide:
        slides[-1]["candidate"] = slides[-2]["candidate"]
    _write_json(
        path,
        {
            "all_slides_meet_95_percent": True,
            "slide_count": declared_slide_count,
            "slides": slides,
        },
    )


def _write_deck(
    root: Path,
    *,
    arrow_payload: str = 'prst="rightArrow"',
    arrow_part: str = "ppt/slides/slide3.xml",
    table_payload: str = "<a:tbl>",
) -> None:
    root.mkdir(parents=True, exist_ok=True)
    with ZipFile(root / "challenge_01.pptx", "w") as archive:
        archive.writestr(arrow_part, arrow_payload)
        archive.writestr("ppt/slides/slide5.xml", table_payload)


def _write_json(path: Path, payload: object) -> None:
    path.write_text(json.dumps(payload), encoding="utf-8")


if __name__ == "__main__":
    unittest.main()
