"""Portable XLSX semantics must match the Rust core exactly.

Both this suite and `crates/document2html-core/tests/spreadsheet_shared_cases_test.rs`
read `evaluate/multiformat/xlsx-semantic-cases.v1.json`, so a divergence in
accept/refuse behaviour or in displayed text fails on one side or the other
rather than going unnoticed.
"""

from __future__ import annotations

import tempfile
import unittest
import zipfile
from dataclasses import dataclass
from pathlib import Path

from evaluate.multiformat_portable_spreadsheet import (
    SpreadsheetSemanticError,
    extract_xlsx_semantics,
)
from evaluate.multiformat_schema import JsonValue, read_object

CASES_PATH = (
    Path(__file__).resolve().parents[1] / "multiformat/xlsx-semantic-cases.v1.json"
)
MAIN = "http://schemas.openxmlformats.org/spreadsheetml/2006/main"
DOC_REL = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
PKG_REL = "http://schemas.openxmlformats.org/package/2006/relationships"
WORKSHEET_TYPE = f"{DOC_REL}/worksheet"


@dataclass(frozen=True, slots=True)
class ExpectedCell:
    address: str
    display: str
    attributable: bool


@dataclass(frozen=True, slots=True)
class SharedCase:
    name: str
    outcome: str
    worksheet_cells: str
    styles: str | None
    relationship_target: str
    relationship_target_mode: str | None
    reason: str | None
    expected: tuple[ExpectedCell, ...]
    unattributed: tuple[str, ...]


def _item(values: list[JsonValue], index: int) -> dict[str, JsonValue]:
    value = values[index]
    if not isinstance(value, dict):
        raise TypeError("expected a JSON object")
    return value


def _text(values: dict[str, JsonValue], key: str) -> str:
    """Reads a string field that may legitimately be empty."""
    value = values.get(key)
    if not isinstance(value, str):
        raise TypeError(f"{key} must be a string")
    return value


def _optional_string(values: dict[str, JsonValue], key: str) -> str | None:
    value = values.get(key)
    if value is None:
        return None
    if not isinstance(value, str):
        raise TypeError(f"{key} must be a string")
    return value


def _expected_cells(value: dict[str, JsonValue]) -> tuple[ExpectedCell, ...]:
    raw = value.get("expected")
    if raw is None:
        return ()
    if not isinstance(raw, list):
        raise TypeError("expected must be a list")
    cells: list[ExpectedCell] = []
    for index in range(len(raw)):
        item = _item(raw, index)
        attributable = item.get("attributable")
        cells.append(
            ExpectedCell(
                address=_text(item, "address"),
                display=_text(item, "display"),
                attributable=True if attributable is None else attributable is True,
            )
        )
    return tuple(cells)


def _unattributed(value: dict[str, JsonValue]) -> tuple[str, ...]:
    raw = value.get("unattributed")
    if raw is None:
        return ()
    if not isinstance(raw, list):
        raise TypeError("unattributed must be a list")
    return tuple(_text(_item(raw, index), "address") for index in range(len(raw)))


def _load_cases() -> tuple[SharedCase, ...]:
    document = read_object(CASES_PATH)
    raw_cases = document.get("cases")
    if not isinstance(raw_cases, list):
        raise TypeError("shared XLSX cases must be a list")
    cases: list[SharedCase] = []
    for index in range(len(raw_cases)):
        case = _item(raw_cases, index)
        cases.append(
            SharedCase(
                name=_text(case, "name"),
                outcome=_text(case, "outcome"),
                worksheet_cells=_text(case, "worksheet_cells"),
                styles=_optional_string(case, "styles"),
                relationship_target=_optional_string(case, "relationship_target")
                or "worksheets/sheet1.xml",
                relationship_target_mode=_optional_string(
                    case, "relationship_target_mode"
                ),
                reason=_optional_string(case, "reason"),
                expected=_expected_cells(case),
                unattributed=_unattributed(case),
            )
        )
    return tuple(cases)


def _write_package(path: Path, case: SharedCase) -> None:
    attributes = (
        f'Id="rId1" Type="{WORKSHEET_TYPE}" Target="{case.relationship_target}"'
    )
    if case.relationship_target_mode is not None:
        attributes += f' TargetMode="{case.relationship_target_mode}"'
    with zipfile.ZipFile(path, "w") as archive:
        if case.styles is not None:
            archive.writestr(
                "xl/styles.xml",
                f'<styleSheet xmlns="{MAIN}">{case.styles}</styleSheet>',
            )
        archive.writestr(
            "xl/workbook.xml",
            f'<workbook xmlns="{MAIN}" xmlns:r="{DOC_REL}"><sheets>'
            f'<sheet name="Sheet1" sheetId="1" r:id="rId1"/>'
            f"</sheets></workbook>",
        )
        archive.writestr(
            "xl/_rels/workbook.xml.rels",
            f'<Relationships xmlns="{PKG_REL}"><Relationship {attributes}/>'
            f"</Relationships>",
        )
        archive.writestr(
            "xl/sharedStrings.xml",
            f'<sst xmlns="{MAIN}"><si><t>Shared</t></si></sst>',
        )
        archive.writestr(
            "xl/worksheets/sheet1.xml",
            f'<worksheet xmlns="{MAIN}"><sheetData><row r="1">'
            f"{case.worksheet_cells}</row></sheetData></worksheet>",
        )


class PortableSpreadsheetSemanticsTests(unittest.TestCase):
    def test_shared_cases_match_the_rust_core(self) -> None:
        cases = _load_cases()
        # Guard against a silently truncated or unparsed fixture.
        self.assertGreaterEqual(len(cases), 20)
        for case in cases:
            with self.subTest(case=case.name):
                with tempfile.TemporaryDirectory() as temp_dir:
                    path = Path(temp_dir) / "case.xlsx"
                    _write_package(path, case)
                    if case.outcome == "refuse":
                        with self.assertRaises(SpreadsheetSemanticError):
                            _ = extract_xlsx_semantics(path)
                        continue
                    value = extract_xlsx_semantics(path)
                    worksheets = value["worksheets"]
                    if not isinstance(worksheets, list):
                        self.fail("worksheets must be a list")
                    worksheet = _item(worksheets, 0)
                    cells = worksheet["cells"]
                    if not isinstance(cells, list):
                        self.fail("cells must be a list")
                    actual = tuple(
                        ExpectedCell(
                            address=_text(item, "address"),
                            display=_text(item, "display"),
                            attributable=item.get("attributable") is True,
                        )
                        for item in (_item(cells, index) for index in range(len(cells)))
                    )
                    self.assertEqual(actual, case.expected)
                    # The refusals must match the shared set exactly, so an
                    # omission can never pass as an absent cell.
                    refused = tuple(
                        _text(item, "address")
                        for item in (
                            _item(raw, index)
                            for raw in [worksheet.get("unattributed_cells") or []]
                            if isinstance(raw, list)
                            for index in range(len(raw))
                        )
                    )
                    self.assertEqual(refused, case.unattributed)

    def test_every_refusal_case_names_a_reason(self) -> None:
        for case in _load_cases():
            if case.outcome == "refuse":
                with self.subTest(case=case.name):
                    self.assertTrue(case.reason)

    def test_percentage_and_iso_date_cases_are_covered(self) -> None:
        # These two regressions are the reason the fixture exists, so their
        # presence is asserted rather than assumed.
        names = {case.name for case in _load_cases()}
        for required in (
            "custom_percent_two_decimals",
            "builtin_percent_no_decimals",
            "builtin_short_date_serial",
            "iso_date_cell_midnight",
            "iso_date_cell_with_time",
            "unreproducible_currency_format_is_unattributable",
        ):
            self.assertIn(required, names)

    def test_percentage_renders_scaled_value_not_raw_fraction(self) -> None:
        case = next(
            item for item in _load_cases() if item.name == "custom_percent_two_decimals"
        )
        with tempfile.TemporaryDirectory() as temp_dir:
            path = Path(temp_dir) / "percent.xlsx"
            _write_package(path, case)

            value = extract_xlsx_semantics(path)

            worksheets = value["worksheets"]
            if not isinstance(worksheets, list):
                self.fail("worksheets must be a list")
            cells = _item(worksheets, 0)["cells"]
            if not isinstance(cells, list):
                self.fail("cells must be a list")
            cell = _item(cells, 0)
            # The stored value is 0.5; the displayed value must not be "0.5".
            self.assertEqual(_text(cell, "display"), "50.00%")


if __name__ == "__main__":
    unittest.main()
