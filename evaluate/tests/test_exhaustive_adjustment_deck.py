import importlib.util
import json
import tempfile
import unittest
from pathlib import Path
from xml.etree import ElementTree
from zipfile import ZipFile

from evaluate import create_exhaustive_adjustment_deck

ROOT = Path(__file__).resolve().parents[2]
ADJUSTMENTS = ROOT / "evaluate" / "preset_adjustments.json"
NAMESPACES = {
    "a": "http://schemas.openxmlformats.org/drawingml/2006/main",
    "p": "http://schemas.openxmlformats.org/presentationml/2006/main",
}


class ExhaustiveAdjustmentDeckTests(unittest.TestCase):
    temporary: tempfile.TemporaryDirectory[str]
    deck: Path
    manifest: Path

    @classmethod
    def setUpClass(cls) -> None:
        cls.temporary = tempfile.TemporaryDirectory()
        output = Path(cls.temporary.name) / "first"
        cls.deck, cls.manifest = (
            create_exhaustive_adjustment_deck.write_exhaustive_adjustment_deck(
                ADJUSTMENTS,
                output,
            )
        )

    @classmethod
    def tearDownClass(cls) -> None:
        cls.temporary.cleanup()

    def test_exhaustive_adjustment_deck_module_exists(self) -> None:
        module = importlib.util.find_spec(
            "evaluate.create_exhaustive_adjustment_deck"
        )

        self.assertIsNotNone(module)

    def test_manifest_covers_all_pairs_and_variants(self) -> None:
        payload = json.loads(self.manifest.read_text(encoding="utf-8"))
        entries = payload["entries"]

        self.assertEqual(payload["adjustment_pair_count"], 300)
        self.assertEqual(payload["case_count"], 900)
        self.assertEqual(payload["slide_count"], 75)
        self.assertEqual(len(entries), 900)
        self.assertEqual(
            sum(payload["range_verification_counts"].values()),
            900,
        )
        self.assertEqual(
            set(payload["range_verification_counts"]),
            {
                "numeric-bounds",
                "default-interpolation",
                "symbolic-unverified",
                "range-unavailable",
            },
        )
        self.assertTrue(
            all("range_verification" in entry for entry in entries)
        )
        self.assertEqual(
            {
                variant
                for entry in entries
                for variant in [entry["variant"]]
            },
            {"low", "default", "high"},
        )
        self.assertEqual(
            len(
                {
                    (entry["preset"], entry["key"], entry["variant"])
                    for entry in entries
                }
            ),
            900,
        )

    def test_every_manifest_case_matches_generated_ooxml(self) -> None:
        payload = json.loads(self.manifest.read_text(encoding="utf-8"))
        expected = {entry["shape_name"]: entry for entry in payload["entries"]}
        actual: dict[str, tuple[str, dict[str, int]]] = {}

        with ZipFile(self.deck) as archive:
            slide_names = sorted(
                name
                for name in archive.namelist()
                if name.startswith("ppt/slides/slide")
                and name.endswith(".xml")
            )
            self.assertEqual(len(slide_names), 75)
            for slide_name in slide_names:
                root = ElementTree.fromstring(archive.read(slide_name))
                shapes = [
                    (shape, "./p:nvSpPr/p:cNvPr")
                    for shape in root.findall(".//p:sp", NAMESPACES)
                ]
                shapes.extend(
                    (shape, "./p:nvCxnSpPr/p:cNvPr")
                    for shape in root.findall(".//p:cxnSp", NAMESPACES)
                )
                for shape, properties_path in shapes:
                    properties = shape.find(properties_path, NAMESPACES)
                    if properties is None:
                        continue
                    shape_name = properties.get("name", "")
                    if not shape_name.startswith("ADJ_"):
                        continue
                    geometry = shape.find("./p:spPr/a:prstGeom", NAMESPACES)
                    self.assertIsNotNone(geometry)
                    adjustments = {
                        guide.get("name", ""): int(
                            guide.get("fmla", "val 0").removeprefix("val ")
                        )
                        for guide in geometry.findall(
                            "./a:avLst/a:gd",
                            NAMESPACES,
                        )
                    }
                    actual[shape_name] = (
                        geometry.get("prst", ""),
                        adjustments,
                    )

        self.assertEqual(set(actual), set(expected))
        for shape_name, entry in expected.items():
            preset, adjustments = actual[shape_name]
            self.assertEqual(preset, entry["preset"])
            self.assertEqual(adjustments, entry["adjustments"])

    def test_clean_generation_is_byte_deterministic(self) -> None:
        second_output = Path(self.temporary.name) / "second"
        second_deck, second_manifest = (
            create_exhaustive_adjustment_deck.write_exhaustive_adjustment_deck(
                ADJUSTMENTS,
                second_output,
            )
        )

        self.assertEqual(self.deck.read_bytes(), second_deck.read_bytes())
        self.assertEqual(
            self.manifest.read_bytes(),
            second_manifest.read_bytes(),
        )

    def test_connector_adjustments_use_connector_shape_elements(self) -> None:
        payload = json.loads(self.manifest.read_text(encoding="utf-8"))
        expected = {
            entry["shape_name"]
            for entry in payload["entries"]
            if "Connector" in entry["preset"]
        }
        actual: set[str] = set()

        with ZipFile(self.deck) as archive:
            for slide_name in archive.namelist():
                if not (
                    slide_name.startswith("ppt/slides/slide")
                    and slide_name.endswith(".xml")
                ):
                    continue
                root = ElementTree.fromstring(archive.read(slide_name))
                for connector in root.findall(".//p:cxnSp", NAMESPACES):
                    properties = connector.find(
                        "./p:nvCxnSpPr/p:cNvPr",
                        NAMESPACES,
                    )
                    if properties is not None:
                        actual.add(properties.get("name", ""))

        self.assertEqual(actual, expected)


if __name__ == "__main__":
    unittest.main()
