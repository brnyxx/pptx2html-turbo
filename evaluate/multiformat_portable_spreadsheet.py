from __future__ import annotations

import zipfile
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
                root = _xml(archive, target)
                cells: list[JsonValue] = []
                unattributed: list[JsonValue] = []
                for element in root.findall(f".//{{{MAIN}}}c"):
                    cell, diagnostic = _cell_value(element, shared, styles)
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


def _cell_value(
    cell: ElementTree.Element,
    shared: list[str],
    styles: list[ResolvedFormat],
) -> tuple[dict[str, JsonValue] | None, dict[str, JsonValue] | None]:
    """Returns `(cell, diagnostic)`; exactly one is set for a present value."""
    address = cell.attrib.get("r", "").replace("$", "")
    if not _valid_coordinate(address):
        raise SpreadsheetSemanticError("XLSX cell coordinate is invalid")
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


def _valid_coordinate(value: str) -> bool:
    """Mirrors the Rust core's A1 reference validation."""
    split = next(
        (index for index, char in enumerate(value) if char.isdigit()), len(value)
    )
    column, row = value[:split], value[split:]
    if not column or len(column) > 3 or not all("A" <= c <= "Z" for c in column):
        return False
    if not row or row.startswith("0") or not row.isdigit():
        return False
    index = 0
    for char in column:
        index = index * 26 + (ord(char) - ord("A") + 1)
    return index <= _MAX_COLUMN and 1 <= int(row) <= _MAX_ROW
