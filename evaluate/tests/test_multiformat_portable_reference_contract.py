from __future__ import annotations

import tempfile
import unittest
import zipfile
from pathlib import Path

from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

from evaluate.multiformat_portable_reference_artifacts import (
    artifact_records,
    load_raw_private_key,
)
from evaluate.multiformat_portable_spreadsheet import (
    SpreadsheetSemanticError,
    extract_xlsx_semantics,
)
from evaluate.multiformat_reference_routing import (
    DocumentFormat,
    load_reference_routing,
)

ROUTING = Path(__file__).resolve().parents[1] / "multiformat/reference-routing.v1.json"


class PortableReferenceContractTests(unittest.TestCase):
    def test_routing_scales_presentations_and_keeps_pages_at_144_dpi(self) -> None:
        routing = load_reference_routing(ROUTING)
        for document_format in (DocumentFormat.PPT, DocumentFormat.PPTX):
            route = next(
                item for item in routing.routes if item.format is document_format
            )
            self.assertEqual(
                route.commands[-2].arguments[1:5],
                ("-scale-to-x", "960", "-scale-to-y", "540"),
            )
        docx = next(
            item for item in routing.routes if item.format is DocumentFormat.DOCX
        )
        self.assertEqual(docx.commands[-2].arguments[1:3], ("-r", "144"))

    def test_xlsx_semantics_preserve_coordinates_and_text(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            path = Path(temp_dir) / "source.xlsx"
            with zipfile.ZipFile(path, "w") as archive:
                archive.writestr(
                    "xl/workbook.xml",
                    '<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Summary" sheetId="1" r:id="rId1"/></sheets></workbook>',
                )
                archive.writestr(
                    "xl/_rels/workbook.xml.rels",
                    '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Target="worksheets/sheet1.xml"/></Relationships>',
                )
                archive.writestr(
                    "xl/sharedStrings.xml",
                    '<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><si><r><t>Alpha</t></r><r><t> Beta</t></r></si></sst>',
                )
                archive.writestr(
                    "xl/worksheets/sheet1.xml",
                    '<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row><c r="A1" t="s"><v>0</v></c><c r="B1" t="inlineStr"><is><t>42</t></is></c></row></sheetData></worksheet>',
                )
            value = extract_xlsx_semantics(path)
            worksheets = value.get("worksheets")
            if not isinstance(worksheets, list) or not worksheets:
                self.fail("spreadsheet semantics must contain worksheets")
            worksheet = worksheets[0]
            if not isinstance(worksheet, dict):
                self.fail("spreadsheet worksheet must be an object")
            self.assertEqual(
                worksheet.get("cells"),
                [
                    {"address": "A1", "display": "Alpha Beta"},
                    {"address": "B1", "display": "42"},
                ],
            )

    def test_xlsx_entities_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            path = Path(temp_dir) / "unsafe.xlsx"
            with zipfile.ZipFile(path, "w") as archive:
                archive.writestr(
                    "xl/workbook.xml", '<!DOCTYPE x [<!ENTITY e "x">]><x/>'
                )
            with self.assertRaisesRegex(SpreadsheetSemanticError, "unsafe"):
                extract_xlsx_semantics(path)

    def test_raw_key_and_sorted_artifacts(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            private = Ed25519PrivateKey.generate()
            key = root / "key.raw"
            key.write_bytes(private.private_bytes_raw())
            key.chmod(0o600)
            first, second = root / "b", root / "a"
            first.write_bytes(b"b")
            second.write_bytes(b"a")
            loaded = load_raw_private_key(key)
            records = artifact_records(root, [(first, "log"), (second, "png")])
            self.assertEqual(
                loaded.public_key().public_bytes_raw(),
                private.public_key().public_bytes_raw(),
            )
            self.assertEqual([item["path"] for item in records], ["a", "b"])

    def test_bad_key_length_fails(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            key = Path(temp_dir) / "key.raw"
            key.write_bytes(b"short")
            with self.assertRaisesRegex(ValueError, "32 bytes"):
                load_raw_private_key(key)


if __name__ == "__main__":
    unittest.main()
