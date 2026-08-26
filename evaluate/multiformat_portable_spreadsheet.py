from __future__ import annotations

import posixpath
import zipfile
from pathlib import Path
from xml.etree import ElementTree

from evaluate.multiformat_schema import JsonValue

MAIN = "http://schemas.openxmlformats.org/spreadsheetml/2006/main"
DOC_REL = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
PKG_REL = "http://schemas.openxmlformats.org/package/2006/relationships"


class SpreadsheetSemanticError(ValueError):
    pass


def extract_xlsx_semantics(path: Path) -> dict[str, JsonValue]:
    try:
        with zipfile.ZipFile(path) as archive:
            workbook = _xml(archive, "xl/workbook.xml")
            relationships = _relationships(archive)
            shared = _shared_strings(archive)
            worksheets: list[JsonValue] = []
            for sheet in workbook.findall(f".//{{{MAIN}}}sheet"):
                name = sheet.attrib.get("name", "")
                relation = sheet.attrib.get(f"{{{DOC_REL}}}id", "")
                target = relationships.get(relation)
                if not name or target is None:
                    raise SpreadsheetSemanticError("XLSX worksheet identity is invalid")
                root = _xml(archive, target)
                cells = [
                    _cell_value(cell, shared)
                    for cell in root.findall(f".//{{{MAIN}}}c")
                ]
                worksheets.append(
                    {
                        "name": name,
                        "cells": [cell for cell in cells if cell is not None],
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
        identity, target = item.attrib.get("Id", ""), item.attrib.get("Target", "")
        normalized = posixpath.normpath(posixpath.join("xl", target))
        if not identity or normalized.startswith("../"):
            raise SpreadsheetSemanticError("XLSX relationship is unsafe")
        result[identity] = normalized
    return result


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
    cell: ElementTree.Element, shared: list[str]
) -> dict[str, JsonValue] | None:
    address = cell.attrib.get("r", "").replace("$", "")
    if not address:
        raise SpreadsheetSemanticError("XLSX cell coordinate is missing")
    kind = cell.attrib.get("t", "n")
    if kind == "inlineStr":
        display = "".join(item.text or "" for item in cell.findall(f".//{{{MAIN}}}t"))
    else:
        value = cell.find(f"{{{MAIN}}}v")
        display = "" if value is None else value.text or ""
        if kind == "s" and display:
            try:
                display = shared[int(display)]
            except (IndexError, ValueError) as error:
                raise SpreadsheetSemanticError(
                    "XLSX shared string is invalid"
                ) from error
        elif kind == "b":
            display = "TRUE" if display == "1" else "FALSE"
    return None if not display else {"address": address, "display": display}
