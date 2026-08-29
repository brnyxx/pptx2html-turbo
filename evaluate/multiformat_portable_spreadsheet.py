from __future__ import annotations

import zipfile
from collections.abc import Iterator
from pathlib import Path
from xml.etree import ElementTree

from evaluate.multiformat_portable_spreadsheet_numbers import ResolvedFormat
from evaluate.multiformat_portable_spreadsheet_formats import (
    UNATTRIBUTABLE,
    cell_format,
    formatted_value,
    iso_date_text,
    parse_styles,
)
from evaluate.multiformat_schema import JsonValue

MAIN = "http://schemas.openxmlformats.org/spreadsheetml/2006/main"
DOC_REL = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
PKG_REL = "http://schemas.openxmlformats.org/package/2006/relationships"
WORKSHEET_REL_SUFFIX = "/relationships/worksheet"
WORKSHEET_PREFIX = "xl/worksheets/"
_MAX_COLUMN = 16_384
_MAX_ROW = 1_048_576
# Cell types whose <v> text is displayed verbatim, matching the Rust core.
_VERBATIM_KINDS = frozenset({"str", "e"})


class SpreadsheetSemanticError(ValueError):
    pass


class _SafeXmlStream:
    """Rejects unsafe declarations while ElementTree incrementally parses XML."""

    def __init__(self, source: zipfile.ZipExtFile) -> None:
        self._source = source
        self._tail = b""

    def read(self, size: int = -1) -> bytes:
        value = self._source.read(size)
        scanned = (self._tail + value).upper()
        if b"<!DOCTYPE" in scanned or b"<!ENTITY" in scanned:
            raise SpreadsheetSemanticError("XLSX XML is unsafe")
        self._tail = scanned[-8:]
        return value


def extract_xlsx_semantics(path: Path) -> dict[str, JsonValue]:
    try:
        with zipfile.ZipFile(path) as archive:
            workbook = _xml(archive, "xl/workbook.xml")
            relationships = _relationships(archive)
            shared = _shared_strings(archive)
            styles = _styles(archive)
            worksheets: list[JsonValue] = []
            for sheet in workbook.findall(f".//{{{MAIN}}}sheet"):
                name = sheet.attrib.get("name", "")
                relation = sheet.attrib.get(f"{{{DOC_REL}}}id", "")
                target = relationships.get(relation)
                if not name or target is None:
                    raise SpreadsheetSemanticError("XLSX worksheet identity is invalid")
                cells: list[JsonValue] = []
                unattributed: list[JsonValue] = []
                for cell, diagnostic in _worksheet_cells(archive, target, shared, styles):
                    if cell is not None:
                        cells.append(cell)
                    if diagnostic is not None:
                        unattributed.append({"worksheet": name, **diagnostic})
                worksheets.append(
                    {
                        "name": name,
                        "cells": cells,
                        # Proof that attribution was skipped for specific
                        # cells, so an omission can never pass unnoticed.
                        "unattributed_cells": unattributed,
                    }
                )
            if not worksheets:
                raise SpreadsheetSemanticError("XLSX workbook has no worksheets")
            return {"worksheets": worksheets}
    except SpreadsheetSemanticError:
        raise
    except (
        KeyError,
        OSError,
        ValueError,
        zipfile.BadZipFile,
        ElementTree.ParseError,
    ) as error:
        raise SpreadsheetSemanticError("XLSX semantic extraction failed") from error


def _xml(archive: zipfile.ZipFile, name: str) -> ElementTree.Element:
    value = archive.read(name)
    upper = value.upper()
    if b"<!DOCTYPE" in upper or b"<!ENTITY" in upper:
        raise SpreadsheetSemanticError("XLSX XML is unsafe")
    return ElementTree.fromstring(value)


def _relationships(archive: zipfile.ZipFile) -> dict[str, str]:
    root = _xml(archive, "xl/_rels/workbook.xml.rels")
    result: dict[str, str] = {}
    for item in root.findall(f"{{{PKG_REL}}}Relationship"):
        kind = item.attrib.get("Type", "")
        if not kind.endswith(WORKSHEET_REL_SUFFIX):
            continue
        # An external target names a resource outside the package and must
        # never be dereferenced, so refuse instead of attempting a read.
        if item.attrib.get("TargetMode") is not None:
            raise SpreadsheetSemanticError("XLSX relationship is external")
        identity, target = item.attrib.get("Id", ""), item.attrib.get("Target", "")
        if not identity or not target:
            raise SpreadsheetSemanticError("XLSX relationship is unsafe")
        if identity in result:
            raise SpreadsheetSemanticError("XLSX relationship id is duplicated")
        result[identity] = _worksheet_path(target)
    return result


def _worksheet_path(target: str) -> str:
    """Mirrors the Rust core's worksheet target resolution.

    Traversal segments are rejected outright rather than normalized away, so a
    target cannot be laundered into an in-package path.
    """
    stripped = target.lstrip("/")
    path = stripped if stripped.startswith("xl/") else f"xl/{stripped}"
    segments = path.split("/")
    if (
        any(not segment or segment == ".." for segment in segments)
        or not path.startswith(WORKSHEET_PREFIX)
        or not path.endswith(".xml")
    ):
        raise SpreadsheetSemanticError("XLSX relationship is unsafe")
    return path


def _styles(archive: zipfile.ZipFile) -> list[ResolvedFormat]:
    try:
        root = _xml(archive, "xl/styles.xml")
    except KeyError:
        return []
    return parse_styles(root)


def _shared_strings(archive: zipfile.ZipFile) -> list[str]:
    try:
        root = _xml(archive, "xl/sharedStrings.xml")
    except KeyError:
        return []
    return [
        "".join(text.text or "" for text in item.findall(f".//{{{MAIN}}}t"))
        for item in root.findall(f"{{{MAIN}}}si")
    ]


def _worksheet_cells(
    archive: zipfile.ZipFile,
    name: str,
    shared: list[str],
    styles: list[ResolvedFormat],
) -> Iterator[tuple[dict[str, JsonValue] | None, dict[str, JsonValue] | None]]:
    current_row: int | None = None
    previous_row: int | None = None
    previous_column: int | None = None
    active_cell_address: str | None = None
    row_tag = f"{{{MAIN}}}row"
    cell_tag = f"{{{MAIN}}}c"
    sheet_data_tag = f"{{{MAIN}}}sheetData"
    sheet_data: ElementTree.Element | None = None
    try:
        with archive.open(name) as source:
            for event, element in ElementTree.iterparse(
                _SafeXmlStream(source), events=("start", "end")
            ):
                if event == "start" and element.tag == sheet_data_tag:
                    sheet_data = element
                elif event == "start" and element.tag == row_tag:
                    if current_row is not None:
                        raise SpreadsheetSemanticError("XLSX row nesting is invalid")
                    current_row = _row_number(element.attrib.get("r"), previous_row)
                    previous_column = None
                elif event == "start" and element.tag == cell_tag:
                    if active_cell_address is not None:
                        raise SpreadsheetSemanticError("XLSX cell nesting is invalid")
                    active_cell_address, previous_column = _cell_coordinate(
                        element.attrib.get("r"), current_row, previous_column
                    )
                elif event == "end" and element.tag == cell_tag:
                    if active_cell_address is None:
                        raise SpreadsheetSemanticError("XLSX cell is invalid")
                    yield _cell_value(element, active_cell_address, shared, styles)
                    active_cell_address = None
                    element.clear()
                elif event == "end" and element.tag == row_tag:
                    if current_row is None or active_cell_address is not None:
                        raise SpreadsheetSemanticError("XLSX row is invalid")
                    previous_row = current_row
                    current_row = None
                    previous_column = None
                    element.clear()
                    if sheet_data is not None:
                        sheet_data.clear()
    except ElementTree.ParseError as error:
        raise SpreadsheetSemanticError("XLSX worksheet is invalid") from error
    if current_row is not None or active_cell_address is not None:
        raise SpreadsheetSemanticError("XLSX worksheet is invalid")


def _cell_value(
    cell: ElementTree.Element,
    address: str,
    shared: list[str],
    styles: list[ResolvedFormat],
) -> tuple[dict[str, JsonValue] | None, dict[str, JsonValue] | None]:
    """Returns `(cell, diagnostic)`; exactly one is set for a present value."""
    kind = cell.attrib.get("t", "n")
    rendered: str | object
    if kind == "inlineStr":
        rendered = "".join(item.text or "" for item in cell.findall(f".//{{{MAIN}}}t"))
    elif kind == "s":
        value = cell.find(f"{{{MAIN}}}v")
        raw = "" if value is None else value.text or ""
        try:
            rendered = shared[int(raw)]
        except (IndexError, ValueError) as error:
            raise SpreadsheetSemanticError("XLSX shared string is invalid") from error
    elif kind == "b":
        value = cell.find(f"{{{MAIN}}}v")
        raw = "" if value is None else value.text or ""
        # ST_Boolean admits only 0 or 1; anything else is malformed rather
        # than falsey, so it must not be coerced to FALSE.
        if raw not in {"0", "1"}:
            raise SpreadsheetSemanticError("XLSX boolean value is invalid")
        rendered = "TRUE" if raw == "1" else "FALSE"
    elif kind == "d":
        # ECMA-376 ISO 8601 date cells convert directly from their text form.
        value = cell.find(f"{{{MAIN}}}v")
        rendered = iso_date_text("" if value is None else value.text or "")
    elif kind in _VERBATIM_KINDS:
        value = cell.find(f"{{{MAIN}}}v")
        rendered = "" if value is None else value.text or ""
    elif kind == "n":
        value = cell.find(f"{{{MAIN}}}v")
        raw = "" if value is None else value.text or ""
        resolved = cell_format(styles, cell.attrib.get("s"))
        rendered = formatted_value(raw, resolved) if raw else ""
        if rendered is UNATTRIBUTABLE:
            return (
                None,
                {
                    "address": address,
                    "stored_value": raw,
                    "number_format": resolved.kind,
                    "reason": "number format display text is not reproduced",
                },
            )
    else:
        raise SpreadsheetSemanticError("XLSX cell type is unsupported")
    # An unreproducible value still converts, but it must not be published as
    # display text. It is reported as a diagnostic so the omission is provable.
    if rendered is UNATTRIBUTABLE:
        return (
            None,
            {
                "address": address,
                "stored_value": _raw_text(cell),
                "number_format": kind,
                "reason": "stored value is not a reproducible display value",
            },
        )
    if not isinstance(rendered, str) or not rendered:
        return (None, None)
    return ({"address": address, "display": rendered, "attributable": True}, None)


def _raw_text(cell: ElementTree.Element) -> str:
    value = cell.find(f"{{{MAIN}}}v")
    return "" if value is None else value.text or ""


def _row_number(value: str | None, previous_row: int | None) -> int:
    if value is None:
        row = (previous_row or 0) + 1
        if row > _MAX_ROW:
            raise SpreadsheetSemanticError("XLSX row coordinate is invalid")
        return row
    row = _parse_row(value)
    if row is None or (previous_row is not None and row <= previous_row):
        raise SpreadsheetSemanticError("XLSX row coordinate is invalid")
    return row


def _cell_coordinate(
    value: str | None, current_row: int | None, previous_column: int | None
) -> tuple[str, int]:
    if current_row is None:
        raise SpreadsheetSemanticError("XLSX cell is outside a row")
    if value is None:
        column = (previous_column or 0) + 1
        if column > _MAX_COLUMN:
            raise SpreadsheetSemanticError("XLSX cell coordinate is invalid")
        return _column_name(column) + str(current_row), column
    coordinate = _parse_coordinate(value)
    if coordinate is None:
        raise SpreadsheetSemanticError("XLSX cell coordinate is invalid")
    column, row = coordinate
    if row != current_row or (previous_column is not None and column <= previous_column):
        raise SpreadsheetSemanticError("XLSX cell coordinate is invalid")
    return value, column


def _parse_coordinate(value: str) -> tuple[int, int] | None:
    """Mirrors the Rust core's explicit A1 reference validation."""
    split = next(
        (index for index, char in enumerate(value) if "0" <= char <= "9"), len(value)
    )
    column, row = value[:split], value[split:]
    if not column or len(column) > 3 or not all("A" <= c <= "Z" for c in column):
        return None
    row_number = _parse_row(row)
    if row_number is None:
        return None
    index = 0
    for char in column:
        index = index * 26 + (ord(char) - ord("A") + 1)
    return (index, row_number) if index <= _MAX_COLUMN else None


def _parse_row(value: str) -> int | None:
    if (
        not value
        or value.startswith("0")
        or not all("0" <= char <= "9" for char in value)
    ):
        return None
    row = int(value)
    return row if 1 <= row <= _MAX_ROW else None


def _column_name(column: int) -> str:
    name = ""
    while column:
        column, remainder = divmod(column - 1, 26)
        name = chr(ord("A") + remainder) + name
    return name
