from __future__ import annotations

import io
import json
import tempfile
import unittest
import zipfile
from pathlib import Path

from evaluate.build_multiformat_conformance_plan import build_conformance_plan
from evaluate.multiformat_conformance_xlsx import (
    XlsxConformanceError,
    inspect_xlsx_package,
    xlsx_case_package,
)

PROJECT_ROOT = Path(__file__).resolve().parents[2]
CONTRACT = PROJECT_ROOT / "evaluate" / "multiformat" / "contract.v1.json"

EXPECTED_ADMISSIONS = {
    "values-formulas": {"cached-values", "formulas", "values"},
    "styles-conditional-formats": {
        "cached-values",
        "conditional-formats",
        "formulas",
        "styles",
        "values",
    },
    "print-layout": {
        "cached-values",
        "formulas",
        "merges",
        "print-settings",
        "sheets",
        "values",
    },
    "charts-images-shapes": {
        "cached-values",
        "charts",
        "drawings",
        "formulas",
        "images",
        "shapes",
        "values",
    },
    "international-formats": {
        "cached-values",
        "formulas",
        "international-formats",
        "values",
    },
    "mixed-stress": {
        "cached-values",
        "charts",
        "conditional-formats",
        "drawings",
        "formulas",
        "images",
        "international-formats",
        "merges",
        "print-settings",
        "shapes",
        "sheets",
        "styles",
        "values",
    },
}


class MultiFormatConformanceXlsxTests(unittest.TestCase):
    def test_admits_every_xlsx_feature_family_from_the_700_case_plan(self) -> None:
        # Given: one planned XLSX case from every contracted stratum
        with tempfile.TemporaryDirectory() as temp_dir:
            plan = Path(temp_dir) / "plan.json"
            build_conformance_plan(CONTRACT, plan)
            values = json.loads(plan.read_text(encoding="utf-8"))
            cases = values["formats"]["xlsx"]["cases"]
            selected = {case["primary_stratum"]: case for case in cases}

            # When: each source is built and structurally inspected
            inspections = {
                stratum: inspect_xlsx_package(xlsx_case_package(case))
                for stratum, case in selected.items()
            }

            # Then: admissions match the contracted feature families
            self.assertEqual(set(inspections), set(EXPECTED_ADMISSIONS))
            for stratum, inspection in inspections.items():
                self.assertEqual(
                    inspection.admissions,
                    frozenset(EXPECTED_ADMISSIONS[stratum]),
                )
                expected_sheets = 2 if "sheets" in inspection.admissions else 1
                self.assertEqual(inspection.sheet_count, expected_sheets)

    def test_package_bytes_and_zip_metadata_are_canonical(self) -> None:
        # Given: a fixed planned identity
        case = {
            "id": "xlsx-conformance-001",
            "ordinal": 1,
            "primary_stratum": "values-formulas",
            "paired_stratum": None,
            "source_kind": "generated-modern",
            "paired_case_id": None,
            "feature_seed": "a" * 64,
        }

        # When: the package is built twice
        first = xlsx_case_package(case)
        second = xlsx_case_package(case)

        # Then: bytes, order, metadata, permissions, and compression are fixed
        self.assertEqual(first, second)
        with zipfile.ZipFile(io.BytesIO(first)) as archive:
            information = archive.infolist()
            inventory = tuple(item.filename for item in information)
            self.assertEqual(inventory, inspect_xlsx_package(first).inventory)
            self.assertTrue(
                all(item.date_time == (1980, 1, 1, 0, 0, 0) for item in information),
            )
            self.assertTrue(all(item.create_system == 3 for item in information))
            self.assertTrue(
                all(item.external_attr == 0o100444 << 16 for item in information),
            )
            self.assertTrue(
                all(item.compress_type == zipfile.ZIP_DEFLATED for item in information),
            )
            worksheet = archive.read("xl/worksheets/sheet1.xml")
            self.assertIn(
                b'<col min="1" max="1" width="28" customWidth="1"/>',
                worksheet,
            )
            self.assertIn(
                b'<col min="2" max="2" width="24" customWidth="1"/>',
                worksheet,
            )
            self.assertIn(
                b'<row r="1" ht="48" customHeight="1">',
                worksheet,
            )
            self.assertIn(
                b'<c r="B1" s="3" t="inlineStr">',
                worksheet,
            )
            styles = archive.read("xl/styles.xml")
            self.assertIn(
                b'<alignment wrapText="1"/>',
                styles,
            )

    def test_rejects_an_invalid_package(self) -> None:
        # Given: bytes that are not an OOXML ZIP package
        # When / Then: structural validation fails closed
        with self.assertRaises(XlsxConformanceError):
            inspect_xlsx_package(b"not-an-xlsx")


if __name__ == "__main__":
    unittest.main()
