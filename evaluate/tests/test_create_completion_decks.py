import hashlib
import json
import subprocess
import sys
import tempfile
import unittest
import zipfile
from pathlib import Path
from typing import TypeAlias
from xml.etree import ElementTree

from evaluate.tests.completion_deck_content_contract import assert_content_types
from evaluate.tests.completion_deck_feature_contract import (
    NS,
    RULES,
    assert_feature_contract,
)
from evaluate.tests.completion_deck_graph_contract import assert_package_graph
from evaluate.tests.completion_deck_test_support import (
    CANONICAL_MANIFEST,
    COMMON_PARTS,
    DECKS,
    ROOT,
    assert_png,
    contract,
    copy_contract,
    generate,
    run_generator,
    tree_hashes,
)


JsonValue: TypeAlias = (
    str | int | bool | None | list["JsonValue"] | dict[str, "JsonValue"]
)


class CompletionDeckTests(unittest.TestCase):
    def test_direct_and_module_cli_use_real_default_manifest(self) -> None:
        checker = subprocess.run(
            [
                sys.executable,
                "evaluate/check_preset_adjustments.py",
                "--repo-root",
                ".",
            ],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(checker.returncode, 0, checker.stderr)
        self.assertIn("presets=187", checker.stdout)
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            direct = run_generator(root / "direct")
            module = run_generator(root / "module", module=True)
            self.assertEqual(direct.returncode, 0, direct.stderr)
            self.assertEqual(module.returncode, 0, module.stderr)
            self.assertEqual(tree_hashes(root / "direct"), tree_hashes(root / "module"))

    def test_corpus_is_deterministic_well_formed_and_graph_closed(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root, source = Path(tmp), contract(Path(tmp))
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
                    for part in (
                        name
                        for name in archive.namelist()
                        if name.endswith((".xml", ".rels"))
                    ):
                        ElementTree.fromstring(archive.read(part))
                    assert_package_graph(self, archive, deck)
                    assert_content_types(self, archive)
            assert_feature_contract(self, first)

    def test_manifest_rows_match_independent_feature_and_real_adjustment_contracts(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / "out"
            generate(self, output, CANONICAL_MANIFEST)
            manifest = json.loads((output / "manifest.json").read_text())
            feature_rows = {row["id"]: row for row in manifest["features"]}
            self.assertEqual(set(feature_rows), {rule.feature_id for rule in RULES})
            for rule in RULES:
                self.assertEqual(
                    feature_rows[rule.feature_id]["deck"], f"{rule.deck}.pptx"
                )
                self.assertEqual(
                    feature_rows[rule.feature_id]["stimulus"]["part"], rule.part
                )
                locator = feature_rows[rule.feature_id]["stimulus"]
                with zipfile.ZipFile(output / f"{rule.deck}.pptx") as archive:
                    self.assertIn(
                        locator["token"].encode(),
                        archive.read(locator["part"]),
                        rule.feature_id,
                    )
            official = json.loads(CANONICAL_MANIFEST.read_text())
            rows = {row["name"]: row for row in official["presets"]}
            cases = manifest["adjustment_case_scaffold"]
            self.assertEqual(len(cases), 12)
            with zipfile.ZipFile(output / "patterns.pptx") as archive:
                slide = ElementTree.fromstring(archive.read("ppt/slides/slide1.xml"))
            for case in cases:
                adjustment = rows[case["preset"]]["adjustments"][0]
                self.assertEqual(case["key"], adjustment["name"])
                self.assertEqual(case["source_status"], adjustment["source_status"])
                shape = next(
                    element
                    for element in slide.findall(".//p:sp", NS)
                    if element.find("p:nvSpPr/p:cNvPr", NS).get("name")
                    == f"adjustment-{case['bundle']}-{case['kind']}"
                )
                geometry = shape.find("p:spPr/a:prstGeom", NS)
                guide = geometry.find("a:avLst/a:gd", NS)
                self.assertEqual(geometry.get("prst"), case["preset"])
                self.assertEqual(guide.get("name"), case["key"])
                formula = str(case["value_or_formula"])
                self.assertEqual(
                    guide.get("fmla"),
                    formula if formula.startswith("val ") else f"val {formula}",
                )
                self.assertIsNone(case["expected_pixels"])

    def test_canonical_manifest_mutations_fail_without_partial_output(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            base = json.loads(copy_contract(root).read_text())
            malicious = 'adj"/><evil injected="yes'
            variants = {
                "missing": {**base, "presets": base["presets"][:-1]},
                "extra": {**base, "presets": [*base["presets"], {"name": "unknown"}]},
                "duplicate": {
                    **base,
                    "presets": [*base["presets"][:-1], base["presets"][0]],
                },
                "unknown": _mutate_adjustment(base, "name", "inventedAdjustment"),
                "mutated": _mutate_adjustment(base, "default_formula", "val 99999"),
                "malicious": _mutate_adjustment(base, "name", malicious),
            }
            for name, payload in variants.items():
                with self.subTest(name=name):
                    source, output = root / f"{name}.json", root / name
                    source.write_text(json.dumps(payload), encoding="utf-8")
                    result = run_generator(output, source)
                    self.assertEqual(result.returncode, 2, result.stderr)
                    self.assertIn("ADJUSTMENT_", result.stderr)
                    self.assertNotIn("Traceback", result.stderr)
                    self.assertFalse(output.exists())

    def test_output_directory_policy_is_stable_and_non_destructive(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            file_output = root / "file"
            file_output.write_text("sentinel", encoding="utf-8")
            nonempty = root / "nonempty"
            nonempty.mkdir()
            sentinel = nonempty / "sentinel.txt"
            sentinel.write_text("keep", encoding="utf-8")
            for output, code in (
                (file_output, "OUTPUT_DIR_NOT_DIRECTORY"),
                (nonempty, "OUTPUT_DIR_NOT_EMPTY"),
            ):
                result = run_generator(output)
                self.assertEqual(result.returncode, 2)
                self.assertIn(code, result.stderr)
                self.assertNotIn("Traceback", result.stderr)
            self.assertEqual(file_output.read_text(), "sentinel")
            self.assertEqual(
                {path.name for path in nonempty.iterdir()}, {"sentinel.txt"}
            )
            canonical_digest = hashlib.sha256(
                CANONICAL_MANIFEST.read_bytes()
            ).hexdigest()
            source_output = run_generator(CANONICAL_MANIFEST.parent)
            self.assertEqual(source_output.returncode, 2)
            self.assertIn("OUTPUT_DIR_NOT_EMPTY", source_output.stderr)
            self.assertEqual(
                hashlib.sha256(CANONICAL_MANIFEST.read_bytes()).hexdigest(),
                canonical_digest,
            )
            empty = root / "empty"
            empty.mkdir()
            result = run_generator(empty)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(len(tuple(empty.iterdir())), 11)

    def test_slide_level_timing_and_png_payloads(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / "out"
            generate(self, output, CANONICAL_MANIFEST)
            with zipfile.ZipFile(output / "timing-transitions.pptx") as archive:
                fade = ElementTree.fromstring(archive.read("ppt/slides/slide1.xml"))
                cut = ElementTree.fromstring(archive.read("ppt/slides/slide2.xml"))
            self.assertIsNotNone(fade.find("p:transition/p:fade", NS))
            self.assertIsNotNone(fade.find("p:timing", NS))
            self.assertIsNotNone(cut.find("p:transition/p:cut", NS))
            self.assertIsNone(fade.find("p:cSld/p:spTree/p:transition", NS))
            for deck in ("picture-bullets", "media", "charts"):
                with zipfile.ZipFile(output / f"{deck}.pptx") as archive:
                    for part in (
                        name for name in archive.namelist() if name.endswith(".png")
                    ):
                        assert_png(self, archive.read(part))


def _mutate_adjustment(
    source: dict[str, JsonValue], field: str, value: str
) -> dict[str, JsonValue]:
    payload = json.loads(json.dumps(source))
    row = next(item for item in payload["presets"] if item["name"] == "roundRect")
    row["adjustments"][0][field] = value
    return payload


if __name__ == "__main__":
    unittest.main()
