import json
import os
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
from evaluate.tests.completion_deck_fixture_contract import assert_fixture_root
from evaluate.tests.completion_deck_locator_contract import assert_manifest_locators
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
    def test_feature_rules_have_no_pseudo_xpath(self) -> None:
        self.assertFalse(
            any(rule.xpath and "not-supported" in rule.xpath for rule in RULES),
            "negative cases must use a typed predicate",
        )

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

    def test_reflection_3d_fixture_is_visible_and_exercises_ordered_fallbacks(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / "out"
            generate(self, output, CANONICAL_MANIFEST)
            with zipfile.ZipFile(output / "reflection-3d.pptx") as archive:
                slide = ElementTree.fromstring(archive.read("ppt/slides/slide1.xml"))
            shapes = {
                shape.find("p:nvSpPr/p:cNvPr", NS).get("name"): shape
                for shape in slide.findall(".//p:sp", NS)
            }
            reflection = shapes["reflection approximate"]
            extent = reflection.find("p:spPr/a:xfrm/a:ext", NS)
            self.assertGreater(int(extent.get("cx", "0")), 0)
            self.assertGreater(int(extent.get("cy", "0")), 0)
            self.assertEqual(
                reflection.findtext(
                    "p:txBody/a:p/a:r/a:t", default="", namespaces=NS
                ),
                "REFLECTION_APPROXIMATE_3D_FALLBACK",
            )
            fallback = shapes["ordered 3d fallback"]
            dag = fallback.find("p:spPr/a:effectDag", NS)
            self.assertEqual(
                [node.get("name") for node in dag.findall("a:cont", NS)],
                ["first", "second"],
            )
            self.assertIsNotNone(fallback.find("p:spPr/a:scene3d", NS))
            self.assertIsNotNone(fallback.find("p:spPr/a:sp3d", NS))

    def test_manifest_rows_match_independent_feature_and_real_adjustment_contracts(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / "out"
            generate(self, output, CANONICAL_MANIFEST)
            manifest = json.loads((output / "manifest.json").read_text())
            feature_rows = {row["id"]: row for row in manifest["features"]}
            assert_manifest_locators(self, feature_rows)
            self.assertTrue({rule.feature_id for rule in RULES} <= set(feature_rows))
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
                    negative = locator.get("negative")
                    if negative:
                        self.assertEqual(negative["kind"], "token_absent")
                        self.assertNotIn(
                            negative["token"].encode(),
                            archive.read(negative["part"]),
                            rule.feature_id,
                        )
            mutated = json.loads(json.dumps(feature_rows))
            mutated["pattern-fill-known"]["stimulus"]["token"] = "<a:pattFill"
            with self.assertRaises(AssertionError):
                assert_manifest_locators(self, mutated)
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


class FixtureRootTests(unittest.TestCase):
    def test_supplied_fixture_root(self) -> None:
        root = os.environ.get("PPTX_COMPLETION_FIXTURE_ROOT")
        self.assertIsNotNone(root)
        assert_fixture_root(self, Path(root))


def load_tests(
    loader: unittest.TestLoader,
    tests: unittest.TestSuite,
    pattern: str | None,
) -> unittest.TestSuite:
    if "PPTX_COMPLETION_FIXTURE_ROOT" not in os.environ:
        return loader.loadTestsFromTestCase(CompletionDeckTests)
    return loader.loadTestsFromTestCase(FixtureRootTests)


def _mutate_adjustment(
    source: dict[str, JsonValue], field: str, value: str
) -> dict[str, JsonValue]:
    payload = json.loads(json.dumps(source))
    row = next(item for item in payload["presets"] if item["name"] == "roundRect")
    row["adjustments"][0][field] = value
    return payload


if __name__ == "__main__":
    unittest.main()
