from __future__ import annotations

import io
import unittest
import zipfile
from xml.etree import ElementTree

from evaluate.multiformat_conformance_docx import docx_case_bytes


class MultiFormatConformanceDocxTests(unittest.TestCase):
    def test_each_stratum_is_admitted_through_observable_ooxml(self) -> None:
        # Given
        strata = (
            "text-typography",
            "sections-headers-footers",
            "tables-images-shapes",
            "lists-fields-references",
            "international",
            "mixed-stress",
        )

        for ordinal, stratum in enumerate(strata, start=1):
            with self.subTest(stratum=stratum):
                case = {
                    "id": f"docx-conformance-{ordinal:03d}",
                    "ordinal": ordinal,
                    "primary_stratum": stratum,
                    "feature_seed": f"{ordinal:064x}",
                }

                # When
                value = docx_case_bytes(case)

                # Then
                with zipfile.ZipFile(io.BytesIO(value)) as archive:
                    names = archive.namelist()
                    self.assertEqual(
                        names,
                        [
                            "[Content_Types].xml",
                            "_rels/.rels",
                            "word/document.xml",
                            "word/styles.xml",
                            "word/numbering.xml",
                            "word/header1.xml",
                            "word/footer1.xml",
                            "word/_rels/document.xml.rels",
                            "word/media/image1.png",
                        ],
                    )
                    self.assertTrue(
                        all(
                            info.date_time == (1980, 1, 1, 0, 0, 0)
                            for info in archive.infolist()
                        )
                    )
                    self.assertTrue(
                        all(
                            info.external_attr >> 16 == 0o100644
                            for info in archive.infolist()
                        )
                    )
                    document = archive.read("word/document.xml")
                    relationships = archive.read("word/_rels/document.xml.rels")
                    self.assertIn(case["id"].encode(), document)
                    self.assertIn(stratum.encode(), document)
                    self.assertIn(b"w:sectPr", document)
                    self.assertIn(b"w:tbl", document)
                    self.assertIn(b"w:numPr", document)
                    self.assertIn(b"w:fldSimple", document)
                    self.assertIn(b"w:drawing", document)
                    self.assertIn(b"v:shape", document)
                    self.assertIn(b"relationships/image", relationships)
                    self.assertIn(b"relationships/header", relationships)
                    self.assertIn(b"relationships/footer", relationships)
                    for name in names[:-1]:
                        if name.endswith((".xml", ".rels")):
                            ElementTree.fromstring(archive.read(name))

    def test_same_case_produces_identical_package_bytes(self) -> None:
        # Given
        case = {
            "id": "docx-conformance-100",
            "ordinal": 100,
            "primary_stratum": "mixed-stress",
            "feature_seed": "ab" * 32,
        }

        # When
        first = docx_case_bytes(case)
        second = docx_case_bytes(case)

        # Then
        self.assertEqual(first, second)


if __name__ == "__main__":
    unittest.main()
