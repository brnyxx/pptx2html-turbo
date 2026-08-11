import json
import os
import subprocess
import sys
import tempfile
import unittest
import zipfile
from pathlib import Path
from xml.etree import ElementTree

from evaluate.tests.completion_deck_test_support import (
    COMMON_PARTS,
    DECKS,
    REQUIRED_IDS,
    ROOT,
    assert_png,
    assert_relationship_closure,
    assert_stimuli,
    contract,
    generate,
    remove_token,
    run_generator,
    tree_hashes,
)


class CompletionDeckTests(unittest.TestCase):
    def test_corpus_is_deterministic_and_structurally_valid(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = contract(root)
            first, second = root / "a", root / "b"
            generate(self, first, source)
            generate(self, second, source)
            expected = {"manifest.json", *(f"{deck}.pptx" for deck in DECKS)}
            self.assertEqual({path.name for path in first.iterdir()}, expected)
            self.assertEqual(tree_hashes(first), tree_hashes(second))
            for deck in DECKS:
                with zipfile.ZipFile(first / f"{deck}.pptx") as archive:
                    self.assertEqual(archive.namelist(), sorted(archive.namelist()))
                    self.assertTrue(COMMON_PARTS.issubset(archive.namelist()))
                    self.assertTrue(
                        all(
                            info.date_time == (1980, 1, 1, 0, 0, 0)
                            for info in archive.infolist()
                        )
                    )
                    assert_relationship_closure(self, archive)

    def test_manifest_features_and_adjustments_bind_to_ooxml(self) -> None:
        fixture_root = os.environ.get("PPTX_COMPLETION_FIXTURE_ROOT")
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            output = Path(fixture_root) if fixture_root else root / "out"
            if not fixture_root:
                generate(self, output, contract(root))
            manifest = json.loads((output / "manifest.json").read_text())
            features = {row["id"]: row for row in manifest["features"]}
            self.assertEqual(set(features), REQUIRED_IDS)
            self.assertTrue(manifest["powerpoint_capture_required"])
            self.assertEqual(
                manifest["native_evidence"], {"images": [], "metadata": None}
            )
            self.assertTrue(
                all(
                    row["native_evidence"] == {"images": [], "metadata": None}
                    for row in features.values()
                )
            )
            assert_stimuli(self, output)
            cases = manifest["adjustment_case_scaffold"]
            self.assertEqual(
                manifest["adjustment_case_source"]["official_preset_count"], 187
            )
            self.assertEqual(len(cases), 12)
            self.assertEqual(
                {case["bundle"] for case in cases}, {"basic", "arrows", "remaining"}
            )
            self.assertEqual(
                {case["kind"] for case in cases},
                {"default", "lower", "upper", "representative"},
            )
            self.assertTrue(all(case["expected_pixels"] is None for case in cases))

    def test_slide_level_timing_and_valid_png_previews(self) -> None:
        ns = {"p": "http://schemas.openxmlformats.org/presentationml/2006/main"}
        with tempfile.TemporaryDirectory() as tmp:
            root, output = Path(tmp), Path(tmp) / "out"
            generate(self, output, contract(root))
            with zipfile.ZipFile(output / "timing-transitions.pptx") as archive:
                fade = ElementTree.fromstring(archive.read("ppt/slides/slide1.xml"))
                cut = ElementTree.fromstring(archive.read("ppt/slides/slide2.xml"))
            self.assertIsNotNone(fade.find("p:transition/p:fade", ns))
            self.assertIsNotNone(fade.find("p:timing", ns))
            self.assertIsNotNone(cut.find("p:transition/p:cut", ns))
            self.assertIsNone(fade.find("p:cSld/p:spTree/p:transition", ns))
            for deck in ("picture-bullets", "media", "charts"):
                with zipfile.ZipFile(output / f"{deck}.pptx") as archive:
                    for part in (
                        name for name in archive.namelist() if name.endswith(".png")
                    ):
                        assert_png(self, archive.read(part))
            with zipfile.ZipFile(output / "charts.pptx") as archive:
                self.assertTrue(
                    {
                        "ppt/charts/chart1.xml",
                        "ppt/charts/chart2.xml",
                        "ppt/charts/chart3.xml",
                    }.issubset(archive.namelist())
                )

    def test_removed_stimulus_and_missing_feature_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root, output = Path(tmp), Path(tmp) / "out"
            generate(self, output, contract(root))
            manifest_path = output / "manifest.json"
            manifest = json.loads(manifest_path.read_text())
            audio = next(
                row for row in manifest["features"] if row["id"] == "media-audio"
            )
            remove_token(output / audio["deck"], audio["stimulus"])
            with self.assertRaisesRegex(AssertionError, "media-audio"):
                assert_stimuli(self, output)
            manifest["features"] = [
                row for row in manifest["features"] if row["id"] != "media-audio"
            ]
            manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
            result = subprocess.run(
                [
                    sys.executable,
                    "-m",
                    "unittest",
                    "evaluate.tests.test_create_completion_decks.CompletionDeckTests.test_manifest_features_and_adjustments_bind_to_ooxml",
                    "-v",
                ],
                cwd=ROOT,
                env={**os.environ, "PPTX_COMPLETION_FIXTURE_ROOT": str(output)},
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("media-audio", result.stderr)

    def test_invalid_adjustment_inventories_fail_stably(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            base = json.loads(contract(root).read_text())
            variants = {
                "missing": {**base, "presets": base["presets"][:-1]},
                "extra": {**base, "presets": [*base["presets"], {"name": "unknown"}]},
                "duplicate": {
                    **base,
                    "presets": [*base["presets"][:-1], base["presets"][0]],
                },
            }
            for name, payload in variants.items():
                path = root / f"{name}.json"
                path.write_text(json.dumps(payload), encoding="utf-8")
                result = run_generator(root / name, path)
                self.assertEqual(result.returncode, 2, name)
                self.assertIn("ADJUSTMENT_INVENTORY_MISMATCH", result.stderr)
                self.assertNotIn("Traceback", result.stderr)


if __name__ == "__main__":
    unittest.main()
