from __future__ import annotations

import tempfile
import unittest
import zipfile
from pathlib import Path
from xml.etree import ElementTree

from evaluate.tests.completion_deck_feature_contract import NS
from evaluate.tests.completion_deck_test_support import CANONICAL_MANIFEST, generate


class PictureBulletCompletionDeckTests(unittest.TestCase):
    def test_picture_bullet_deck_has_independent_size_and_failure_stimuli(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / "output"
            generate(self, output, CANONICAL_MANIFEST)

            with zipfile.ZipFile(output / "picture-bullets.pptx") as archive:
                slide = ElementTree.fromstring(archive.read("ppt/slides/slide1.xml"))
                relationships = ElementTree.fromstring(
                    archive.read("ppt/slides/_rels/slide1.xml.rels")
                )
                content_types = ElementTree.fromstring(
                    archive.read("[Content_Types].xml")
                )

                paragraphs = {
                    paragraph.find("a:r/a:t", NS).text: paragraph
                    for paragraph in slide.findall(".//a:p", NS)
                    if paragraph.find("a:r/a:t", NS) is not None
                }
                self.assertIsNotNone(paragraphs["Size text"].find("a:pPr/a:buSzTx", NS))
                self.assertEqual(
                    paragraphs["Size 25"].find("a:pPr/a:buSzPct", NS).get("val"),
                    "25000",
                )
                self.assertEqual(
                    paragraphs["Size 400"].find("a:pPr/a:buSzPct", NS).get("val"),
                    "400000",
                )
                self.assertEqual(
                    paragraphs["Size points"].find("a:pPr/a:buSzPts", NS).get("val"),
                    "1250",
                )
                self.assertIsNone(
                    paragraphs["Missing reference"].find("a:pPr/a:buBlip/a:blip", NS).get(
                        f"{{{NS['r']}}}embed"
                    )
                )
                self.assertEqual(
                    paragraphs["Wrong kind"].find("a:pPr/a:buBlip/a:blip", NS).get(
                        f"{{{NS['r']}}}embed"
                    ),
                    "rIdWrongKind",
                )
                self.assertEqual(
                    paragraphs["Unsupported SVG"].find("a:pPr/a:buBlip/a:blip", NS).get(
                        f"{{{NS['r']}}}embed"
                    ),
                    "rIdSvg",
                )
                self.assertEqual(
                    paragraphs["Linked external"].find("a:pPr/a:buBlip/a:blip", NS).get(
                        f"{{{NS['r']}}}link"
                    ),
                    "rIdLinked",
                )

                rels = {relationship.get("Id"): relationship for relationship in relationships}
                self.assertEqual(rels["rIdImage"].get("Type"), f"{NS['r']}/image")
                self.assertEqual(rels["rIdWrongKind"].get("Type"), f"{NS['r']}/chart")
                self.assertEqual(rels["rIdLinked"].get("TargetMode"), "External")
                self.assertNotIn("rIdMissing", rels)
                overrides = {
                    element.get("PartName"): element.get("ContentType")
                    for element in content_types
                    if element.tag.endswith("Override")
                }
                self.assertEqual(overrides["/ppt/media/bullet.png"], "image/png")
                self.assertEqual(overrides["/ppt/media/bullet.svg"], "image/svg+xml")
                self.assertTrue(archive.read("ppt/media/bullet.png").startswith(b"\x89PNG"))
                self.assertIn(b"<script>", archive.read("ppt/media/bullet.svg"))


if __name__ == "__main__":
    unittest.main()
