from __future__ import annotations

import hashlib
import io
import json
import shutil
import subprocess
import sys
import tempfile
import unittest
import zipfile
from collections.abc import Callable
from pathlib import Path

from evaluate.check_preset_adjustments import (
    JsonValue,
    check_repository,
    verify_official_artifact,
)


REPO_ROOT = Path(__file__).resolve().parents[2]
CHECKER = REPO_ROOT / "evaluate/check_preset_adjustments.py"
MANIFEST = REPO_ROOT / "evaluate/preset_adjustments.json"
GEOMETRY_SOURCE = REPO_ROOT / "crates/pptx2html-core/src/renderer/geometry"


class CheckPresetAdjustmentsTests(unittest.TestCase):
    def _run_cli(self, *args: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, str(CHECKER), "--repo-root", str(REPO_ROOT), *args],
            check=False,
            capture_output=True,
            text=True,
        )

    def _copy_source(self, root: Path) -> Path:
        source_root = root / "geometry"
        shutil.copytree(GEOMETRY_SOURCE, source_root)
        return source_root

    def _mutated_manifest(
        self,
        root: Path,
        mutate: Callable[[dict[str, JsonValue]], None],
    ) -> Path:
        payload = json.loads(MANIFEST.read_text(encoding="utf-8"))
        mutate(payload)
        root.mkdir(parents=True, exist_ok=True)
        path = root / "manifest.json"
        path.write_text(json.dumps(payload), encoding="utf-8")
        return path

    def _official_artifact(
        self,
        root: Path,
        names: list[str],
    ) -> tuple[Path, str, str]:
        root.mkdir(parents=True, exist_ok=True)
        enums = "".join(f'<xsd:enumeration value="{name}"/>' for name in names)
        xsd = (
            '<xsd:schema xmlns:xsd="http://www.w3.org/2001/XMLSchema">'
            '<xsd:simpleType name="ST_ShapeType"><xsd:restriction base="xsd:token">'
            f"{enums}</xsd:restriction></xsd:simpleType></xsd:schema>"
        ).encode()
        nested_buffer = io.BytesIO()
        with zipfile.ZipFile(nested_buffer, "w") as nested:
            nested.writestr("dml-main.xsd", xsd)
        artifact = root / "part1.zip"
        with zipfile.ZipFile(artifact, "w") as outer:
            outer.writestr(
                "OfficeOpenXML-XMLSchema-Strict.zip", nested_buffer.getvalue()
            )
        return (
            artifact,
            hashlib.sha256(artifact.read_bytes()).hexdigest(),
            hashlib.sha256(xsd).hexdigest(),
        )

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
        self.assertNotEqual(presets["upArrow"]["preservation"]["fidelity"], "exact")
        self.assertEqual(
            manifest["custom_geometry_adjustments"]["name_contract"], "open"
        )

    def test_cli_writes_json_report(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            output = Path(tmpdir) / "report.json"
            result = self._run_cli("--json", str(output))
            self.assertEqual(result.returncode, 0, result.stderr)
            report = json.loads(output.read_text(encoding="utf-8"))
            self.assertEqual(
                (report["presets"], report["unknown_consumed_keys"]), (187, 0)
            )

    def test_official_inventory_list_and_rows_reject_missing_extra_or_alias(
        self,
    ) -> None:
        def replace_official(payload: dict[str, JsonValue]) -> None:
            names = payload["official_preset_names"]
            names[names.index("foldedCorner")] = "foldCorner"

        def replace_row(payload: dict[str, JsonValue]) -> None:
            row = next(
                row for row in payload["presets"] if row["name"] == "foldedCorner"
            )
            row["name"] = "foldCorner"

        def remove_row(payload: dict[str, JsonValue]) -> None:
            payload["presets"] = [
                row for row in payload["presets"] if row["name"] != "foldedCorner"
            ]

        def add_row(payload: dict[str, JsonValue]) -> None:
            payload["presets"].append(
                {"name": "foldCorner", "adjustments": [], "preservation": {}}
            )

        with tempfile.TemporaryDirectory() as tmpdir:
            for index, mutate in enumerate(
                (replace_official, replace_row, remove_row, add_row)
            ):
                with self.subTest(mutate=mutate.__name__):
                    path = self._mutated_manifest(Path(tmpdir) / str(index), mutate)
                    result = self._run_cli("--manifest", str(path))
                    self.assertNotEqual(result.returncode, 0)
                    self.assertIn("OFFICIAL_PRESET_INVENTORY_MISMATCH", result.stderr)

    def test_lookup_scanner_ignores_comments_and_strings(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            source_root = self._copy_source(Path(tmpdir))
            source = source_root / "basic_shapes.rs"
            text = source.read_text(encoding="utf-8").replace(
                ") -> String {",
                ') -> String {\n// adj.get("commentAdj")\nlet _fake = "adj.get(\\"stringAdj\\")";',
                1,
            )
            source.write_text(text, encoding="utf-8")
            result = self._run_cli("--source-root", str(source_root))
            self.assertEqual(result.returncode, 0, result.stderr)

    def test_lookup_scanner_detects_multiline_and_unrouted_keys(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            source_root = self._copy_source(Path(tmpdir))
            source = source_root / "basic_shapes.rs"
            text = source.read_text(encoding="utf-8")
            multiline = ".get(\n" + " " * 64 + '"inventedAdj"\n)'
            text = text.replace('.get("adj")', multiline, 1)
            text += '\nfn unused(adj: &HashMap<String, f64>) { let _ = adj.get("unroutedAdj"); }\n'
            source.write_text(text, encoding="utf-8")
            result = self._run_cli("--source-root", str(source_root))
            self.assertNotEqual(result.returncode, 0)
            self.assertIn(
                "preset=roundRect family=basic_shapes key=inventedAdj", result.stderr
            )
            self.assertIn(
                "preset=<unrouted> family=basic_shapes key=unroutedAdj", result.stderr
            )

    def test_official_artifact_verifies_nested_xsd_checksum_and_inventory(self) -> None:
        names = json.loads(MANIFEST.read_text(encoding="utf-8"))[
            "official_preset_names"
        ]
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            artifact, outer_sha, xsd_sha = self._official_artifact(root, names)

            def update_artifact(payload: dict[str, JsonValue]) -> None:
                payload["official_artifact"]["sha256"] = outer_sha
                payload["official_artifact"]["inventory_member_sha256"] = xsd_sha

            manifest = self._mutated_manifest(root / "valid", update_artifact)
            valid = self._run_cli(
                "--manifest", str(manifest), "--official-artifact", str(artifact)
            )
            self.assertEqual(valid.returncode, 0, valid.stderr)
            payload = json.loads(manifest.read_text())
            payload["official_artifact"]["inventory_member_sha256"] = "0" * 64
            manifest.write_text(json.dumps(payload))
            invalid = self._run_cli(
                "--manifest", str(manifest), "--official-artifact", str(artifact)
            )
            self.assertNotEqual(invalid.returncode, 0)
            self.assertIn("OFFICIAL_ARTIFACT_CHECKSUM_MISMATCH", invalid.stderr)
            altered = names.copy()
            altered[altered.index("foldedCorner")] = "foldCorner"
            bad_artifact, bad_outer_sha, bad_xsd_sha = self._official_artifact(
                root / "altered", altered
            )
            payload["official_artifact"]["sha256"] = bad_outer_sha
            payload["official_artifact"]["inventory_member_sha256"] = bad_xsd_sha
            manifest.write_text(json.dumps(payload))
            mismatch = self._run_cli(
                "--manifest", str(manifest), "--official-artifact", str(bad_artifact)
            )
            self.assertNotEqual(mismatch.returncode, 0)
            self.assertIn("OFFICIAL_PRESET_INVENTORY_MISMATCH", mismatch.stderr)

    def test_cli_reports_manifest_and_source_contract_errors_without_traceback(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            malformed = root / "malformed.json"
            malformed.write_text("{")
            directory_manifest = root / "manifest-dir"
            directory_manifest.mkdir()
            cases = (
                (("--manifest", str(malformed)), "MALFORMED_MANIFEST"),
                (("--manifest", str(root / "missing.json")), "MANIFEST_NOT_FOUND"),
                (("--manifest", str(directory_manifest)), "MANIFEST_UNREADABLE"),
                (
                    ("--source-root", str(root / "missing-source")),
                    "INVALID_SOURCE_ROOT",
                ),
                (("--dispatcher", str(root / "missing.rs")), "INVALID_DISPATCHER"),
            )
            for args, code in cases:
                with self.subTest(code=code):
                    result = self._run_cli(*args)
                    self.assertEqual(result.returncode, 1)
                    self.assertIn(code, result.stderr)
                    self.assertNotIn("Traceback", result.stderr)

    def test_official_artifact_checksum_is_verified(self) -> None:
        expected = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        with tempfile.TemporaryDirectory() as tmpdir:
            artifact = Path(tmpdir) / "artifact.zip"
            artifact.write_bytes(b"abc")
            self.assertEqual(verify_official_artifact(artifact, expected), expected)


if __name__ == "__main__":
    unittest.main()
