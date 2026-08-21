from __future__ import annotations

import io
import json
import stat
import tempfile
import unittest
import zipfile
from collections import Counter
from pathlib import Path
from xml.etree import ElementTree

from evaluate.build_multiformat_conformance_plan import build_conformance_plan
from evaluate.generate_multiformat_xlsx_conformance import generate_xlsx_conformance
from evaluate.multiformat_conformance_xlsx import (
    XlsxConformanceError,
    inspect_xlsx_package,
    xlsx_case_package,
)
from evaluate.multiformat_schema import JsonValue, sha256_file

PROJECT_ROOT = Path(__file__).resolve().parents[2]
CONTRACT = PROJECT_ROOT / "evaluate" / "multiformat" / "contract.v1.json"
CHART_NAMESPACE = "http://schemas.openxmlformats.org/drawingml/2006/chart"
EXTENDED_NAMESPACE = (
    "http://schemas.openxmlformats.org/officeDocument/2006/extended-properties"
)
VARIANT_NAMESPACE = (
    "http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes"
)
EXPECTED_QUOTAS = {
    "values-formulas": 25,
    "styles-conditional-formats": 20,
    "print-layout": 20,
    "charts-images-shapes": 15,
    "international-formats": 10,
    "mixed-stress": 10,
}


class GenerateMultiFormatXlsxConformanceTests(unittest.TestCase):
    def test_admission_rejects_invalid_ooxml_root_tags(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            plan = Path(temp_dir) / "plan.json"
            build_conformance_plan(CONTRACT, plan)
            basic = xlsx_case_package(_case_for_stratum(plan, "values-formulas"))
            graphical = xlsx_case_package(
                _case_for_stratum(plan, "charts-images-shapes")
            )
            mutations = (
                (basic, "[Content_Types].xml", b"Types", b"Typos"),
                (basic, "_rels/.rels", b"Relationships", b"Relatxionships"),
                (basic, "xl/workbook.xml", b"workbook", b"workboox"),
                (basic, "xl/worksheets/sheet1.xml", b"worksheet", b"worksheez"),
                (
                    graphical,
                    "xl/drawings/drawing1.xml",
                    b"xdr:wsDr",
                    b"xdr:wsDx",
                ),
                (
                    graphical,
                    "xl/charts/chart1.xml",
                    b"c:chartSpace",
                    b"c:chartSpacz",
                ),
            )

            for package, part, old, new in mutations:
                with self.subTest(part=part):
                    mutated = _replace_part(package, part, old, new)
                    with self.assertRaises(XlsxConformanceError):
                        inspect_xlsx_package(mutated)

    def test_admission_rejects_package_contract_mutations(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            plan = Path(temp_dir) / "plan.json"
            build_conformance_plan(CONTRACT, plan)
            case = _case_for_stratum(plan, "values-formulas")
            package = xlsx_case_package(case)
            mutations = (
                (
                    "[Content_Types].xml",
                    b"spreadsheetml.sheet.main+xml",
                    b"spreadsheetml.sheet.main+bad",
                ),
                (
                    "_rels/.rels",
                    b"/officeDocument",
                    b"/officeDocumenz",
                ),
                (
                    "xl/_rels/workbook.xml.rels",
                    b"/worksheet",
                    b"/workshaat",
                ),
                (
                    "xl/workbook.xml",
                    b'r:id="rId1"',
                    b'r:id="rId9"',
                ),
            )

            for part, old, new in mutations:
                with self.subTest(part=part, old=old):
                    mutated = _replace_part(package, part, old, new)
                    with self.assertRaises(XlsxConformanceError):
                        inspect_xlsx_package(mutated)

    def test_chart_axes_include_scaling(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            plan = Path(temp_dir) / "plan.json"
            build_conformance_plan(CONTRACT, plan)
            package = xlsx_case_package(_case_for_stratum(plan, "charts-images-shapes"))

            with zipfile.ZipFile(io.BytesIO(package)) as archive:
                chart = ElementTree.fromstring(archive.read("xl/charts/chart1.xml"))

            for axis_name in ("catAx", "valAx"):
                with self.subTest(axis=axis_name):
                    axis = chart.find(
                        f".//{{{CHART_NAMESPACE}}}{axis_name}",
                    )
                    self.assertIsNotNone(axis)
                    assert axis is not None
                    self.assertIsNotNone(
                        axis.find(f"{{{CHART_NAMESPACE}}}scaling"),
                    )

    def test_titles_of_parts_vector_matches_sheets(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            plan = Path(temp_dir) / "plan.json"
            build_conformance_plan(CONTRACT, plan)
            package = xlsx_case_package(_case_for_stratum(plan, "print-layout"))

            with zipfile.ZipFile(io.BytesIO(package)) as archive:
                properties = ElementTree.fromstring(archive.read("docProps/app.xml"))

            vector = properties.find(
                f".//{{{EXTENDED_NAMESPACE}}}TitlesOfParts/"
                f"{{{VARIANT_NAMESPACE}}}vector",
            )
            self.assertIsNotNone(vector)
            assert vector is not None
            self.assertEqual(vector.attrib, {"size": "2", "baseType": "lpstr"})
            self.assertEqual(
                [item.text for item in vector],
                ["Conformance", "Print Area"],
            )

    def test_materializes_frozen_100_source_snapshot(self) -> None:
        # Given: the contract-derived 700-case plan
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            plan = root / "plan.json"
            build_conformance_plan(CONTRACT, plan)

            # When: the XLSX source snapshot is materialized
            manifest = generate_xlsx_conformance(CONTRACT, plan, root / "snapshot")
            values = json.loads(manifest.read_text(encoding="utf-8"))
            files = values["files"]

            # Then: exactly 100 immutable sources satisfy exact strata and bindings
            self.assertEqual(len(files), 100)
            self.assertEqual(
                Counter(item["primary_stratum"] for item in files),
                EXPECTED_QUOTAS,
            )
            self.assertEqual(
                [item["ordinal"] for item in files],
                list(range(1, 101)),
            )
            self.assertTrue(all(item["feature_seed"] for item in files))
            self.assertTrue(all(item["package_inventory"] for item in files))
            self.assertTrue(all(item["admissions"] for item in files))
            self.assertEqual(values["contract_sha256"], sha256_file(CONTRACT))
            self.assertEqual(values["plan_sha256"], sha256_file(plan))
            self.assertTrue(values["frozen_snapshot"]["normative"])
            self.assertFalse(values["generator_reproducibility"]["normative"])
            self.assertEqual(
                values["portable_evaluation"],
                {
                    "office_engine": "LibreOffice",
                    "pdf_engine": "Poppler",
                },
            )
            self.assertEqual(
                values["source_sha256"],
                {item["id"]: item["sha256"] for item in files},
            )
            actual_files = [
                path for path in manifest.parent.rglob("*") if path.is_file()
            ]
            self.assertEqual(len(actual_files), 101)
            self.assertTrue(
                all(
                    stat.S_IMODE(path.stat().st_mode) == 0o444 for path in actual_files
                ),
            )

    def test_reproduces_snapshot_identity_in_a_clean_output(self) -> None:
        # Given: one exact plan and two absent output roots
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            plan = root / "plan.json"
            build_conformance_plan(CONTRACT, plan)

            # When: independent materializations are performed
            first = generate_xlsx_conformance(CONTRACT, plan, root / "first")
            second = generate_xlsx_conformance(CONTRACT, plan, root / "second")
            first_values = json.loads(first.read_text(encoding="utf-8"))
            second_values = json.loads(second.read_text(encoding="utf-8"))

            # Then: reproducibility confirms but does not define frozen identity
            self.assertEqual(
                first_values["frozen_snapshot"]["identity_sha256"],
                second_values["frozen_snapshot"]["identity_sha256"],
            )
            self.assertEqual(first_values, second_values)

    def test_refuses_existing_output_and_leaves_failed_output_absent(self) -> None:
        # Given: an existing destination and a non-conforming plan
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            plan = root / "plan.json"
            build_conformance_plan(CONTRACT, plan)
            existing = root / "existing"
            existing.mkdir()
            invalid_plan = root / "invalid.json"
            invalid_plan.write_text("{}\n", encoding="utf-8")
            failed = root / "failed"

            # When / Then: output refusal preserves existing data
            with self.assertRaises(XlsxConformanceError):
                generate_xlsx_conformance(CONTRACT, plan, existing)
            self.assertEqual(list(existing.iterdir()), [])

            # When / Then: failed generation cleans every staged artifact
            with self.assertRaises(XlsxConformanceError):
                generate_xlsx_conformance(CONTRACT, invalid_plan, failed)
            self.assertFalse(failed.exists())
            self.assertEqual(list(root.glob(".xlsx-conformance-*")), [])


def _case_for_stratum(plan: Path, stratum: str) -> dict[str, JsonValue]:
    values = json.loads(plan.read_text(encoding="utf-8"))
    return next(
        case
        for case in values["formats"]["xlsx"]["cases"]
        if case["primary_stratum"] == stratum
    )


def _replace_part(value: bytes, part: str, old: bytes, new: bytes) -> bytes:
    source = zipfile.ZipFile(io.BytesIO(value))
    output = io.BytesIO()
    with source, zipfile.ZipFile(output, "w") as target:
        for information in source.infolist():
            data = source.read(information)
            if information.filename == part:
                if old not in data:
                    raise AssertionError(f"{old!r} missing from {part}")
                data = data.replace(old, new)
            target.writestr(information, data)
    return output.getvalue()


if __name__ == "__main__":
    unittest.main()
