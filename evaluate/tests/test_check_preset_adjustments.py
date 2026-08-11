from __future__ import annotations

import json
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from evaluate.check_preset_adjustments import (
    UNKNOWN_ADJUSTMENT_KEY,
    check_repository,
    verify_official_artifact,
)


REPO_ROOT = Path(__file__).resolve().parents[2]
CHECKER = REPO_ROOT / "evaluate/check_preset_adjustments.py"
MANIFEST = REPO_ROOT / "evaluate/preset_adjustments.json"
GEOMETRY_SOURCE = REPO_ROOT / "crates/pptx2html-core/src/renderer/geometry"


class CheckPresetAdjustmentsTests(unittest.TestCase):
    def test_repository_classifies_all_official_presets(self) -> None:
        report = check_repository(REPO_ROOT)

        self.assertTrue(report["ok"])
        self.assertEqual(report["presets"], 187)
        self.assertEqual(report["unclassified_presets"], 0)
        self.assertEqual(report["unknown_consumed_keys"], 0)
        self.assertGreater(report["non_official_consumed_keys_preserved"], 0)
        self.assertGreater(report["manifest_keys_never_consumed"], 0)

    def test_manifest_keeps_unavailable_source_and_custom_names_separate(self) -> None:
        manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
        presets = {row["name"]: row for row in manifest["presets"]}

        self.assertEqual(presets["upArrow"]["source_status"], "unavailable")
        self.assertNotEqual(
            presets["upArrow"]["preservation"]["fidelity"],
            "exact",
        )
        self.assertEqual(
            manifest["custom_geometry_adjustments"]["name_contract"],
            "open",
        )

    def test_cli_writes_json_report(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            output = Path(tmpdir) / "report.json"
            result = subprocess.run(
                [
                    sys.executable,
                    str(CHECKER),
                    "--repo-root",
                    str(REPO_ROOT),
                    "--json",
                    str(output),
                ],
                check=False,
                capture_output=True,
                text=True,
            )

            self.assertEqual(result.returncode, 0, result.stderr)
            report = json.loads(output.read_text(encoding="utf-8"))
            self.assertEqual(report["presets"], 187)
            self.assertEqual(report["unknown_consumed_keys"], 0)

    def test_source_root_override_rejects_invented_adjustment_key(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            source_root = Path(tmpdir) / "geometry"
            shutil.copytree(GEOMETRY_SOURCE, source_root)
            basic_shapes = source_root / "basic_shapes.rs"
            source = basic_shapes.read_text(encoding="utf-8")
            basic_shapes.write_text(
                source.replace('.get("adj")', '.get("inventedAdj")', 1),
                encoding="utf-8",
            )
            result = subprocess.run(
                [
                    sys.executable,
                    str(CHECKER),
                    "--repo-root",
                    str(REPO_ROOT),
                    "--source-root",
                    str(source_root),
                ],
                check=False,
                capture_output=True,
                text=True,
            )

            self.assertNotEqual(result.returncode, 0)
            self.assertIn(UNKNOWN_ADJUSTMENT_KEY, result.stderr)
            self.assertIn("roundRect", result.stderr)
            self.assertIn("basic_shapes", result.stderr)
            self.assertIn("inventedAdj", result.stderr)

    def test_source_root_override_checks_unrouted_adjustment_lookups(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            source_root = Path(tmpdir) / "geometry"
            shutil.copytree(GEOMETRY_SOURCE, source_root)
            basic_shapes = source_root / "basic_shapes.rs"
            source = basic_shapes.read_text(encoding="utf-8")
            basic_shapes.write_text(
                source
                + '\nfn unused(adj: &HashMap<String, f64>) { let _ = adj.get("inventedAdj"); }\n',
                encoding="utf-8",
            )
            result = subprocess.run(
                [
                    sys.executable,
                    str(CHECKER),
                    "--repo-root",
                    str(REPO_ROOT),
                    "--source-root",
                    str(source_root),
                ],
                check=False,
                capture_output=True,
                text=True,
            )

            self.assertNotEqual(result.returncode, 0)
            self.assertIn(UNKNOWN_ADJUSTMENT_KEY, result.stderr)
            self.assertIn("preset=<unrouted>", result.stderr)
            self.assertIn("inventedAdj", result.stderr)

    def test_official_artifact_checksum_is_verified(self) -> None:
        expected = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        with tempfile.TemporaryDirectory() as tmpdir:
            artifact = Path(tmpdir) / "artifact.zip"
            artifact.write_bytes(b"abc")

            self.assertEqual(verify_official_artifact(artifact, expected), expected)


if __name__ == "__main__":
    unittest.main()
