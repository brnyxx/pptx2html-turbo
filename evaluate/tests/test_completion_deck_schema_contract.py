from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
import zipfile
from dataclasses import replace
from pathlib import Path
from unittest import mock
from xml.etree import ElementTree

from evaluate import create_completion_decks
from evaluate.completion_deck_features import FEATURES
from evaluate.completion_deck_inventory import validate_features
from evaluate.completion_deck_manifest import ContractError
from evaluate.tests.completion_deck_feature_contract import NS
from evaluate.tests.completion_deck_inventory_contract import assert_inventory
from evaluate.tests.completion_deck_test_support import (
    CANONICAL_MANIFEST,
    ROOT,
    generate,
    run_generator,
)


CANONICAL_FEATURES = ROOT / "evaluate" / "completeness_manifest.json"


class CompletionDeckSchemaTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.temp = tempfile.TemporaryDirectory()
        cls.root = Path(cls.temp.name) / "corpus"
        generate(cls(), cls.root, CANONICAL_MANIFEST)

    @classmethod
    def tearDownClass(cls) -> None:
        cls.temp.cleanup()

    def test_common_graph_schema_minima(self) -> None:
        for deck in self.root.glob("*.pptx"):
            with zipfile.ZipFile(deck) as archive:
                master = ElementTree.fromstring(
                    archive.read("ppt/slideMasters/slideMaster1.xml")
                )
                layout_id = master.find("p:sldLayoutIdLst/p:sldLayoutId", NS)
                self.assertGreaterEqual(int(layout_id.get("id")), 2147483648, deck.name)
                theme = ElementTree.fromstring(archive.read("ppt/theme/theme1.xml"))
                for collection in ("majorFont", "minorFont"):
                    node = theme.find(
                        f"a:themeElements/a:fontScheme/a:{collection}", NS
                    )
                    self.assertEqual(
                        [child.tag.rsplit("}", 1)[-1] for child in node],
                        ["latin", "ea", "cs"],
                        deck.name,
                    )
                fmt = theme.find("a:themeElements/a:fmtScheme", NS)
                for name in (
                    "fillStyleLst",
                    "lnStyleLst",
                    "effectStyleLst",
                    "bgFillStyleLst",
                ):
                    self.assertGreaterEqual(
                        len(fmt.find(f"a:{name}", NS)), 3, deck.name
                    )

    def test_notes_and_comments_are_independent_and_closed(self) -> None:
        with zipfile.ZipFile(self.root / "notes-comments.pptx") as archive:
            names = set(archive.namelist())
            required = {
                "ppt/notesMasters/notesMaster1.xml",
                "ppt/notesMasters/_rels/notesMaster1.xml.rels",
                "ppt/theme/notesTheme1.xml",
                "ppt/notesSlides/_rels/notesSlide1.xml.rels",
                "ppt/commentAuthors.xml",
            }
            self.assertTrue(required <= names)
            slide = ElementTree.fromstring(archive.read("ppt/slides/slide1.xml"))
            visible = next(
                (
                    shape
                    for shape in slide.findall(".//p:sp", NS)
                    if shape.find("p:nvSpPr/p:cNvPr", NS).get("name")
                    == "visible body"
                ),
                None,
            )
            self.assertIsNotNone(visible)
            self.assertEqual(
                visible.findtext("p:txBody/a:p/a:r/a:t", namespaces=NS),
                "VISIBLE_SLIDE_BODY",
            )
            extent = visible.find("p:spPr/a:xfrm/a:ext", NS)
            self.assertGreater(int(extent.get("cx")), 0)
            self.assertGreater(int(extent.get("cy")), 0)
            modern = ElementTree.fromstring(
                archive.read("ppt/comments/modernComment1.xml")
            ).find("p188:cm", NS)
            self.assertEqual(modern[0].tag, f"{{{NS['p188']}}}unknownAnchor")
            extension = modern.find("p188:extLst/p:ext", NS)
            self.assertEqual(extension.get("uri"), "fixture-modern-extension")
            payload = extension[0]
            self.assertEqual(
                payload.tag, "{urn:pptx2html:fixture:future}payload"
            )
            self.assertEqual(
                payload.text, "MODERN_EXTENSION_SENTINEL</script>"
            )
            classic = ElementTree.fromstring(archive.read("ppt/comments/comment1.xml"))
            comments = classic.findall("p:cm", NS)
            self.assertEqual([row.get("authorId") for row in comments], ["0", "404"])
            authors = ElementTree.fromstring(archive.read("ppt/commentAuthors.xml"))
            self.assertIsNotNone(authors.find("p:cmAuthor[@id='0']", NS))
            self.assertIsNone(authors.find("p:cmAuthor[@id='404']", NS))

    def test_charts_have_required_series_axes_and_preview_bounds(self) -> None:
        with zipfile.ZipFile(self.root / "charts.pptx") as archive:
            expectations = (
                (1, "barChart", 1, 2),
                (2, "surface3DChart", 1, 3),
                (3, "stockChart", 3, 2),
            )
            for index, kind, series, axes in expectations:
                root = ElementTree.fromstring(
                    archive.read(f"ppt/charts/chart{index}.xml")
                )
                chart = root.find(f".//c:{kind}", NS)
                self.assertEqual(len(chart.findall("c:ser", NS)), series)
                self.assertEqual(len(chart.findall("c:axId", NS)), axes)
                defined = (
                    root.findall(".//c:catAx", NS)
                    + root.findall(".//c:valAx", NS)
                    + root.findall(".//c:serAx", NS)
                )
                self.assertEqual(len(defined), axes)
            self.assertIn("ppt/charts/_rels/chart2.xml.rels", archive.namelist())
            self.assertNotIn("ppt/charts/_rels/chart3.xml.rels", archive.namelist())

    def test_table_style_part_contains_positive_and_excludes_unavailable(self) -> None:
        with zipfile.ZipFile(self.root / "table-styles.pptx") as archive:
            styles = ElementTree.fromstring(archive.read("ppt/tableStyles.xml"))
            custom = styles.find("a:tblStyle", NS)
            self.assertEqual(
                custom.get("styleId"), "{11111111-1111-1111-1111-111111111111}"
            )
            for region in ("wholeTbl", "band1H", "firstRow", "lastCol", "nwCell"):
                self.assertIsNotNone(custom.find(f"a:{region}", NS))
            self.assertNotIn(
                "{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}",
                archive.read("ppt/tableStyles.xml").decode(),
            )

    def test_actions_use_nonsequential_parts_in_presentation_order(self) -> None:
        with zipfile.ZipFile(self.root / "actions.pptx") as archive:
            presentation = ElementTree.fromstring(archive.read("ppt/presentation.xml"))
            rels = ElementTree.fromstring(
                archive.read("ppt/_rels/presentation.xml.rels")
            )
            targets = {
                rel.get("Id"): rel.get("Target")
                for rel in rels.findall("pr:Relationship", NS)
            }
            order = [
                targets[node.get(f"{{{NS['r']}}}id")]
                for node in presentation.findall("p:sldIdLst/p:sldId", NS)
            ]
            self.assertEqual(
                order,
                ["slides/slide1.xml", "slides/slide42.xml", "slides/slide7.xml"],
            )
            slide_rels = ElementTree.fromstring(
                archive.read("ppt/slides/_rels/slide1.xml.rels")
            )
            specific = slide_rels.find("pr:Relationship[@Id='rIdSpecific']", NS)
            self.assertEqual(specific.get("Target"), "slide7.xml")
            slide = ElementTree.fromstring(archive.read("ppt/slides/slide1.xml"))
            self.assertIsNotNone(
                slide.find(".//a:prstGeom[@prst='actionButtonForwardNext']", NS)
            )
            group = next(
                item
                for item in slide.findall(".//p:grpSp", NS)
                if item.find("p:nvGrpSpPr/p:cNvPr", NS).get("name")
                == "outer action group"
            )
            group_properties = group.find("p:nvGrpSpPr/p:cNvPr", NS)
            self.assertEqual(
                group_properties.find("a:hlinkClick", NS).get(f"{{{NS['r']}}}id"),
                "rIdExternal",
            )
            self.assertEqual(
                group_properties.find("a:hlinkMouseOver", NS).get("action"),
                "ppaction://hlinkshowjump?jump=lastslide",
            )
            table = next(
                item
                for item in slide.findall(".//p:graphicFrame", NS)
                if item.find("p:nvGraphicFramePr/p:cNvPr", NS).get("name")
                == "action table"
            )
            table_properties = table.find("p:nvGraphicFramePr/p:cNvPr", NS)
            self.assertEqual(
                table_properties.find("a:hlinkClick", NS).get(f"{{{NS['r']}}}id"),
                "rIdExternal",
            )
            self.assertEqual(
                table_properties.find("a:hlinkMouseOver", NS).get("action"),
                "ppaction://program",
            )
            table_run = table.find(".//a:rPr/a:hlinkClick", NS)
            self.assertEqual(table_run.get(f"{{{NS['r']}}}id"), "rIdMailto")

    def test_missing_table_style_fixture_preserves_id_and_flags(self) -> None:
        with zipfile.ZipFile(self.root / "table-styles.pptx") as archive:
            slide = ElementTree.fromstring(archive.read("ppt/slides/slide1.xml"))
            missing = next(
                frame
                for frame in slide.findall(".//p:graphicFrame", NS)
                if frame.find("p:nvGraphicFramePr/p:cNvPr", NS).get("name")
                == "missing style"
            )
            properties = missing.find(".//a:tblPr", NS)
            self.assertEqual(
                {name: properties.get(name) for name in ("firstCol", "bandCol")},
                {"firstCol": "1", "bandCol": "1"},
            )
            style_id = properties.find("a:tableStyleId", NS)
            self.assertIsNotNone(
                style_id,
                "missing-definition fallback must preserve the referenced style ID",
            )
            self.assertEqual(
                style_id.text,
                "{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}",
            )

    def test_table_style_fixture_has_region_override_and_merge_matrix(self) -> None:
        with zipfile.ZipFile(self.root / "table-styles.pptx") as archive:
            slide = ElementTree.fromstring(archive.read("ppt/slides/slide1.xml"))
            present = next(
                frame
                for frame in slide.findall(".//p:graphicFrame", NS)
                if frame.find("p:nvGraphicFramePr/p:cNvPr", NS).get("name")
                == "present style"
            )
            rows = present.findall(".//a:tr", NS)
            self.assertEqual(len(rows), 5)
            self.assertEqual(len(rows[0].findall("a:tc", NS)), 4)
            explicit = rows[1].findall("a:tc", NS)
            self.assertIsNotNone(explicit[1].find("a:tcPr/a:solidFill", NS))
            self.assertIsNotNone(explicit[2].find("a:tcPr/a:noFill", NS))
            merged = rows[-1].findall("a:tc", NS)
            self.assertEqual(merged[0].get("gridSpan"), "2")
            self.assertEqual(merged[1].get("hMerge"), "1")

    def test_fallback_domains_close_diagram_ole_and_mc(self) -> None:
        with zipfile.ZipFile(self.root / "fallback-domains.pptx") as archive:
            slide = ElementTree.fromstring(archive.read("ppt/slides/slide1.xml"))
            rel_ids = slide.find(".//dgm:relIds", NS)
            self.assertEqual(
                {
                    rel_ids.get(f"{{{NS['r']}}}{name}")
                    for name in ("dm", "lo", "qs", "cs")
                },
                {
                    "rIdDiagramData",
                    "rIdDiagramLayout",
                    "rIdDiagramStyle",
                    "rIdDiagramColors",
                },
            )
            ole = slide.find(".//p:oleObj", NS)
            self.assertIsNotNone(ole.find("p:embed", NS))
            choice = slide.find(".//mc:Choice", NS)
            self.assertEqual(choice.get("Requires"), "x14")
            self.assertIn(
                b'xmlns:x14="http://schemas.microsoft.com/office/drawing/2010/main"',
                archive.read("ppt/slides/slide1.xml"),
            )
            for part in ("data1.xml", "layout1.xml", "quickStyle1.xml", "colors1.xml"):
                self.assertIn(f"ppt/diagrams/{part}", archive.namelist())

    def test_manifest_joins_canonical_inventory_and_bounds_negatives(self) -> None:
        manifest = json.loads((self.root / "manifest.json").read_text())
        assert_inventory(self, manifest, CANONICAL_FEATURES)

    def test_scenario_and_canonical_inventory_mutations_are_rejected(self) -> None:
        variants = (
            ("DUPLICATE", (*FEATURES, FEATURES[0])),
            ("MISMATCH", FEATURES[:-1]),
            ("MISMATCH", (*FEATURES, replace(FEATURES[0], feature_id="invented"))),
        )
        for code, features in variants:
            with self.subTest(code=code), tempfile.TemporaryDirectory() as tmp:
                output = Path(tmp) / "out"
                with mock.patch.object(create_completion_decks, "FEATURES", features):
                    with self.assertRaisesRegex(ContractError, code):
                        create_completion_decks.generate(output, CANONICAL_MANIFEST)
                self.assertFalse(output.exists())
        with tempfile.TemporaryDirectory() as tmp:
            canonical = json.loads(CANONICAL_FEATURES.read_text())
            canonical["features"] = [
                row for row in canonical["features"] if row["id"] != "media-audio"
            ]
            path = Path(tmp) / "canonical.json"
            path.write_text(json.dumps(canonical), encoding="utf-8")
            with self.assertRaisesRegex(
                ContractError, "CANONICAL_UNKNOWN.*media-audio"
            ):
                validate_features(FEATURES, path)

    def test_fixture_root_missing_media_audio_fails_named(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            fixture = Path(tmp) / "fixture"
            shutil.copytree(self.root, fixture)
            manifest_path = fixture / "manifest.json"
            manifest = json.loads(manifest_path.read_text())
            manifest["features"] = [
                row for row in manifest["features"] if row["id"] != "media-audio"
            ]
            manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
            env = {**os.environ, "PPTX_COMPLETION_FIXTURE_ROOT": str(fixture)}
            result = subprocess.run(
                [
                    sys.executable,
                    "-m",
                    "unittest",
                    "evaluate.tests.test_create_completion_decks",
                    "-v",
                ],
                cwd=ROOT,
                env=env,
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("media-audio", result.stdout + result.stderr)

    def test_existing_empty_output_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / "existing"
            output.mkdir()
            result = run_generator(output)
            self.assertEqual(result.returncode, 2)
            self.assertIn("OUTPUT_DIR_EXISTS", result.stderr)
            self.assertFalse(any(output.iterdir()))


if __name__ == "__main__":
    unittest.main()
