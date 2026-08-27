from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from evaluate.multiformat_inventory import parse_inventory
from evaluate.multiformat_office_oracle_batch import (
    OfficeOracleBatchFile,
    OfficeOracleBatchUnit,
)
from evaluate.multiformat_office_oracle_inventory import (
    OfficeOracleInventoryError,
    write_office_oracle_inventories,
)
from evaluate.tests.multiformat_metric_artifact_fixture import write_png


class MultiFormatOfficeOracleInventoryTests(unittest.TestCase):
    def test_spreadsheet_cells_keep_worksheet_coordinate_and_page_geometry(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source = self._spreadsheet_source(root)

            paths = write_office_oracle_inventories(
                source,
                ["xlsx-unit-1", "xlsx-unit-2"],
                root / "inventories",
            )

            first = parse_inventory(paths[0], "xlsx-unit-1")
            second = parse_inventory(paths[1], "xlsx-unit-2")
            self.assertEqual(first.cells[0].worksheet, "Sheet1")
            self.assertEqual(first.cells[0].coordinate, "A1")
            self.assertEqual(second.cells[0].coordinate, "A2")
            self.assertEqual(first.cells[0].box.width, 100)

    def test_unattributed_cells_are_surfaced_in_every_inventory(self) -> None:
        """An attribution refusal must reach the inventory as evidence.

        Omitting it would make an unsupported number format indistinguishable
        from a cell that never existed.
        """
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source = self._spreadsheet_source(root, unattributed=True)

            paths = write_office_oracle_inventories(
                source,
                ["xlsx-unit-1", "xlsx-unit-2"],
                root / "inventories",
            )

            for index, path in enumerate(paths, start=1):
                inventory = parse_inventory(path, f"xlsx-unit-{index}")
                self.assertEqual(len(inventory.unattributed_cells), 1)
                refusal = inventory.unattributed_cells[0]
                self.assertEqual(refusal.worksheet, "Sheet1")
                self.assertEqual(refusal.address, "B9")
                self.assertEqual(refusal.number_format, "unsupported")

    def test_layout_entities_are_rejected_before_parsing(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source = self._spreadsheet_source(root)
            source.layout.write_text(
                '<!DOCTYPE doc [<!ENTITY x "bad">]><doc>&x;</doc>',
                encoding="utf-8",
            )

            with self.assertRaisesRegex(
                OfficeOracleInventoryError,
                "unsafe",
            ):
                write_office_oracle_inventories(
                    source,
                    ["xlsx-unit-1", "xlsx-unit-2"],
                    root / "inventories",
                )

    def test_fixed_poppler_xhtml_doctype_is_accepted(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source = self._spreadsheet_source(root)
            value = source.layout.read_text(encoding="utf-8")
            source.layout.write_text(
                '<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.0 Transitional//EN" '
                '"http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd">'
                f'<html xmlns="http://www.w3.org/1999/xhtml"><body>{value}</body></html>',
                encoding="utf-8",
            )

            paths = write_office_oracle_inventories(
                source,
                ["xlsx-unit-1", "xlsx-unit-2"],
                root / "inventories",
            )

            self.assertEqual(len(paths), 2)

    def _spreadsheet_source(
        self,
        root: Path,
        *,
        unattributed: bool = False,
    ) -> OfficeOracleBatchFile:
        semantic = root / "semantic.json"
        worksheet: dict[str, object] = {
            "name": "Sheet1",
            "cells": [
                {"address": "$A$1", "display": "Alpha"},
                {"address": "$A$2", "display": "Beta"},
            ],
        }
        if unattributed:
            worksheet["unattributed_cells"] = [
                {
                    "worksheet": "Sheet1",
                    "address": "B9",
                    "number_format": "unsupported",
                    "reason": "number format display text is not reproduced",
                }
            ]
        semantic.write_text(
            json.dumps({"worksheets": [worksheet]}, sort_keys=True),
            encoding="utf-8",
        )
        layout = root / "layout.xml"
        layout.write_text(
            "<doc>" + _page("Alpha") + _page("Beta") + "</doc>",
            encoding="utf-8",
        )
        pdf = root / "reference.pdf"
        pdf.write_bytes(b"%PDF-1.7\n%%EOF\n")
        units = []
        for index in range(2):
            png = root / f"page-{index + 1}.png"
            write_png(png, 200, 200, (1, 2, 3))
            units.append(OfficeOracleBatchUnit(png, 200, 200))
        return OfficeOracleBatchFile(
            "source",
            "xlsx",
            "0" * 64,
            pdf,
            semantic,
            layout,
            tuple(units),
        )


def _page(value: str) -> str:
    return (
        '<page width="200" height="200"><flow><block>'
        '<line xMin="10" yMin="10" xMax="110" yMax="30">'
        f'<word xMin="10" yMin="10" xMax="110" yMax="30">{value}</word>'
        "</line></block></flow></page>"
    )
