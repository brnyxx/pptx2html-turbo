from __future__ import annotations

import tempfile
import unittest
import zipfile
from pathlib import Path
from typing import Final
from xml.etree import ElementTree

from evaluate.tests.completion_deck_feature_contract import NS
from evaluate.tests.completion_deck_test_support import CANONICAL_MANIFEST, generate


OFFICIAL_PATTERNS: Final = (
    "pct5",
    "pct10",
    "pct20",
    "pct25",
    "pct30",
    "pct40",
    "pct50",
    "pct60",
    "pct70",
    "pct75",
    "pct80",
    "pct90",
    "horz",
    "vert",
    "ltHorz",
    "ltVert",
    "dkHorz",
    "dkVert",
    "narHorz",
    "narVert",
    "dashHorz",
    "dashVert",
    "cross",
    "dnDiag",
    "upDiag",
    "ltDnDiag",
    "ltUpDiag",
    "dkDnDiag",
    "dkUpDiag",
    "wdDnDiag",
    "wdUpDiag",
    "dashDnDiag",
    "dashUpDiag",
    "diagCross",
    "smCheck",
    "lgCheck",
    "smGrid",
    "lgGrid",
    "dotGrid",
    "smConfetti",
    "lgConfetti",
    "horzBrick",
    "diagBrick",
    "solidDmnd",
    "openDmnd",
    "dotDmnd",
    "plaid",
    "sphere",
    "weave",
    "divot",
    "shingle",
    "wave",
    "trellis",
    "zigZag",
)


class CompletionDeckPatternTests(unittest.TestCase):
    def test_patterns_deck_contains_exact_labeled_official_inventory_and_surfaces(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / "out"
            generate(self, output, CANONICAL_MANIFEST)
            with zipfile.ZipFile(output / "patterns.pptx") as archive:
                slides = tuple(
                    ElementTree.fromstring(archive.read(f"ppt/slides/slide{index}.xml"))
                    for index in range(1, 4)
                )

        shape_presets: list[str] = []
        shape_labels: list[str] = []
        for slide in slides:
            for shape in slide.findall(".//p:sp", NS):
                pattern = shape.find("p:spPr/a:pattFill", NS)
                if pattern is None or pattern.get("prst") == "unknownFuturePattern":
                    continue
                shape_presets.append(pattern.get("prst", ""))
                shape_labels.append(shape.find("p:nvSpPr/p:cNvPr", NS).get("name", ""))
                self.assertEqual(
                    pattern.find("a:fgClr/a:srgbClr", NS).get("val"), "4472C4"
                )
                self.assertEqual(
                    pattern.find("a:bgClr/a:srgbClr", NS).get("val"), "F2F2F2"
                )

        self.assertEqual(tuple(shape_presets), OFFICIAL_PATTERNS)
        self.assertEqual(
            tuple(shape_labels), tuple(f"pattern-{name}" for name in OFFICIAL_PATTERNS)
        )
        self.assertEqual(len(set(shape_presets)), 54)
        unknown = slides[0].find(
            ".//p:sp/p:spPr/a:pattFill[@prst='unknownFuturePattern']", NS
        )
        self.assertIsNotNone(unknown)
        self.assertIsNotNone(
            slides[1].find("p:cSld/p:bg/p:bgPr/a:pattFill[@prst='trellis']", NS)
        )
        self.assertIsNotNone(
            slides[2].find(".//a:tcPr/a:pattFill[@prst='diagCross']", NS)
        )


if __name__ == "__main__":
    unittest.main()
