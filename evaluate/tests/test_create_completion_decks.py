import hashlib
import json
import os
import subprocess
import sys
import tempfile
import unittest
import zipfile
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
GENERATOR = REPO_ROOT / "evaluate" / "create_completion_decks.py"
DECK_NAMES = (
    "patterns",
    "picture-bullets",
    "table-styles",
    "actions",
    "notes-comments",
    "reflection-3d",
    "media",
    "timing-transitions",
    "charts",
    "fallback-domains",
)
REQUIRED_FEATURE_IDS = {
    "adjustment-basic",
    "adjustment-arrows",
    "adjustment-remaining",
    "custom-geometry-unknown-formula",
    "pattern-fill-known",
    "pattern-fill-unknown",
    "picture-bullet-embedded",
    "picture-bullet-missing",
    "table-style-regions",
    "table-style-missing",
    "action-external",
    "action-internal",
    "action-unsafe",
    "notes-slide",
    "comments-legacy",
    "comments-modern",
    "comment-author-missing",
    "reflection",
    "drawingml-3d-fallback",
    "media-audio",
    "media-video",
    "media-unsupported",
    "transition-cut",
    "transition-fade",
    "animation-bounded",
    "animation-unsupported",
    "chart-direct",
    "chart-preview-fallback",
    "chart-placeholder",
    "fallback-smartart",
    "fallback-ole",
    "fallback-math",
    "fallback-alternate-content",
    "fallback-unknown-extension",
}


class CompletionDeckTests(unittest.TestCase):
    def test_generates_required_deterministic_corpus(self) -> None:
        with tempfile.TemporaryDirectory() as first_tmp, tempfile.TemporaryDirectory() as second_tmp:
            first = Path(first_tmp)
            second = Path(second_tmp)

            self._generate(first)
            self._generate(second)

            expected_names = {"manifest.json", *(f"{name}.pptx" for name in DECK_NAMES)}
            self.assertEqual({path.name for path in first.iterdir()}, expected_names)
            self.assertEqual(self._tree_hashes(first), self._tree_hashes(second))

    def test_packages_have_fixed_zip_metadata_and_required_parts(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            output_dir = Path(tmpdir)
            self._generate(output_dir)

            for deck_name in DECK_NAMES:
                with self.subTest(deck=deck_name), zipfile.ZipFile(output_dir / f"{deck_name}.pptx") as archive:
                    names = archive.namelist()
                    self.assertEqual(names, sorted(names))
                    self.assertTrue({"[Content_Types].xml", "_rels/.rels", "ppt/presentation.xml", "ppt/slides/slide1.xml"}.issubset(names))
                    self.assertTrue(all(info.date_time == (1980, 1, 1, 0, 0, 0) for info in archive.infolist()))

    def test_manifest_covers_planned_features_without_native_evidence(self) -> None:
        fixture_root = os.environ.get("PPTX_COMPLETION_FIXTURE_ROOT")
        if fixture_root:
            manifest_path = Path(fixture_root) / "manifest.json"
        else:
            temporary = tempfile.TemporaryDirectory()
            self.addCleanup(temporary.cleanup)
            output_dir = Path(temporary.name)
            self._generate(output_dir)
            manifest_path = output_dir / "manifest.json"

        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        features = {row["id"]: row for row in manifest["features"]}

        self.assertTrue(manifest["powerpoint_capture_required"])
        self.assertEqual(manifest["native_evidence"], {"images": [], "metadata": None})
        self.assertEqual(set(features), REQUIRED_FEATURE_IDS)
        self.assertTrue(all(row["powerpoint_capture_required"] for row in features.values()))
        self.assertTrue(all(row["native_evidence"] == {"images": [], "metadata": None} for row in features.values()))
        self.assertEqual(
            [case["kind"] for case in manifest["adjustment_case_scaffold"]],
            ["default", "lower", "upper", "representative"],
        )
        self.assertEqual(manifest["adjustment_case_source"]["status"], "awaiting-task-2-manifest")

    def _generate(self, output_dir: Path) -> None:
        subprocess.run(
            [sys.executable, str(GENERATOR), "--output-dir", str(output_dir)],
            cwd=REPO_ROOT,
            check=True,
            capture_output=True,
            text=True,
        )

    def _tree_hashes(self, root: Path) -> dict[str, str]:
        return {
            path.name: hashlib.sha256(path.read_bytes()).hexdigest()
            for path in sorted(root.iterdir())
            if path.is_file()
        }


if __name__ == "__main__":
    unittest.main()
