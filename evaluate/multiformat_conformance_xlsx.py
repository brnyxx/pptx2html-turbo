from __future__ import annotations

import io
import posixpath
import re
import zipfile
from dataclasses import dataclass
from pathlib import PurePosixPath
from xml.etree import ElementTree

from evaluate.multiformat_corpus_items import require_keys
from evaluate.multiformat_schema import (
    JsonValue,
    integer_value,
    sha256_value,
    string_value,
)
from evaluate.multiformat_xlsx_features import XlsxCase, XlsxStratum, feature_flags
from evaluate.multiformat_xlsx_package_validation import validate_package_contract
from evaluate.multiformat_xlsx_parts import package_parts

_FIXED_TIME = (1980, 1, 1, 0, 0, 0)
_FIXED_MODE = 0o100444 << 16
_MAIN_NS = "http://schemas.openxmlformats.org/spreadsheetml/2006/main"
_REL_NS = "http://schemas.openxmlformats.org/package/2006/relationships"
_DRAWING_NS = "http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing"


@dataclass(frozen=True, slots=True)
class XlsxConformanceError(Exception):
    reason: str

    def __str__(self) -> str:
        return self.reason


@dataclass(frozen=True, slots=True)
class XlsxInspection:
    inventory: tuple[str, ...]
    admissions: frozenset[str]
    sheet_count: int


@dataclass(frozen=True, slots=True)
class _PackageView:
    names: tuple[str, ...]
    parts: dict[str, bytes]
    roots: dict[str, ElementTree.Element]


def xlsx_case_package(case_value: dict[str, JsonValue]) -> bytes:
    case = _parse_case(case_value)
    flags = feature_flags(case.stratum)
    output = io.BytesIO()
    with zipfile.ZipFile(output, "w") as archive:
        for name, value in package_parts(case, flags):
            information = zipfile.ZipInfo(name, _FIXED_TIME)
            information.compress_type = zipfile.ZIP_DEFLATED
            information.create_system = 3
            information.external_attr = _FIXED_MODE
            archive.writestr(
                information,
                value,
                compress_type=zipfile.ZIP_DEFLATED,
                compresslevel=9,
            )
    package = output.getvalue()
    inspection = inspect_xlsx_package(package)
    if inspection.admissions != flags.admissions:
        raise XlsxConformanceError("XLSX feature admissions differ")
    return package


def inspect_xlsx_package(value: bytes) -> XlsxInspection:
    try:
        with zipfile.ZipFile(io.BytesIO(value)) as archive:
            information = archive.infolist()
            names = tuple(item.filename for item in information)
            if len(names) != len(set(names)) or archive.testzip() is not None:
                raise XlsxConformanceError("XLSX ZIP inventory is invalid")
            _validate_zip_metadata(information)
            parts = {name: archive.read(name) for name in names}
        _validate_inventory(names)
        view = _PackageView(names, parts, _parse_xml_parts(parts))
        _validate_relationships(view)
        validate_package_contract(view.names, view.roots)
        return _inspect_features(view)
    except XlsxConformanceError:
        raise
    except (
        ElementTree.ParseError,
        KeyError,
        OSError,
        RuntimeError,
        UnicodeError,
        ValueError,
        zipfile.BadZipFile,
    ) as error:
        raise XlsxConformanceError("XLSX package validation failed") from error


def _parse_case(values: dict[str, JsonValue]) -> XlsxCase:
    require_keys(
        values,
        {
            "id",
            "ordinal",
            "primary_stratum",
            "paired_stratum",
            "source_kind",
            "paired_case_id",
            "feature_seed",
        },
        "xlsx.case",
    )
    case_id = string_value(values, "id")
    ordinal = integer_value(values, "ordinal")
    if not re.fullmatch(r"xlsx-conformance-\d{3}", case_id) or ordinal < 1:
        raise XlsxConformanceError("XLSX planned identity is invalid")
    try:
        stratum = XlsxStratum(string_value(values, "primary_stratum"))
    except ValueError as error:
        raise XlsxConformanceError("XLSX stratum is unsupported") from error
    return XlsxCase(
        case_id=case_id,
        ordinal=ordinal,
        stratum=stratum,
        feature_seed=sha256_value(values, "feature_seed"),
    )


def _validate_zip_metadata(information: list[zipfile.ZipInfo]) -> None:
    if any(
        item.date_time != _FIXED_TIME
        or item.create_system != 3
        or item.external_attr != _FIXED_MODE
        or item.compress_type != zipfile.ZIP_DEFLATED
        or item.flag_bits & 1
        for item in information
    ):
        raise XlsxConformanceError("XLSX ZIP metadata differs")


def _validate_inventory(names: tuple[str, ...]) -> None:
    printable = "xl/worksheets/sheet2.xml" in names
    graphical = "xl/drawings/drawing1.xml" in names
    expected = [
        "[Content_Types].xml",
        "_rels/.rels",
        "docProps/core.xml",
        "docProps/app.xml",
        "xl/workbook.xml",
        "xl/_rels/workbook.xml.rels",
        "xl/styles.xml",
        "xl/worksheets/sheet1.xml",
    ]
    if graphical:
        expected.append("xl/worksheets/_rels/sheet1.xml.rels")
    if printable:
        expected.append("xl/worksheets/sheet2.xml")
    if graphical:
        expected.extend(
            (
                "xl/drawings/drawing1.xml",
                "xl/drawings/_rels/drawing1.xml.rels",
                "xl/charts/chart1.xml",
                "xl/media/image1.png",
            ),
        )
    if names != tuple(expected):
        raise XlsxConformanceError("XLSX package inventory differs")


def _parse_xml_parts(
    parts: dict[str, bytes],
) -> dict[str, ElementTree.Element]:
    return {
        name: ElementTree.fromstring(value)
        for name, value in parts.items()
        if name.endswith((".xml", ".rels"))
    }


def _validate_relationships(view: _PackageView) -> None:
    for name, root in view.roots.items():
        if not name.endswith(".rels"):
            continue
        relationships = root.findall(f"{{{_REL_NS}}}Relationship")
        ids = [relationship.attrib.get("Id") for relationship in relationships]
        if ids != [f"rId{index}" for index in range(1, len(ids) + 1)]:
            raise XlsxConformanceError("XLSX relationship IDs differ")
        base = _relationship_base(name)
        for relationship in relationships:
            target = relationship.attrib.get("Target", "")
            resolved = posixpath.normpath(posixpath.join(base, target))
            if (
                not target
                or relationship.attrib.get("TargetMode") is not None
                or resolved.startswith("../")
                or resolved not in view.parts
            ):
                raise XlsxConformanceError("XLSX relationship target is invalid")


def _relationship_base(name: str) -> str:
    if name == "_rels/.rels":
        return ""
    owner = name.replace("/_rels/", "/").removesuffix(".rels")
    return PurePosixPath(owner).parent.as_posix()


def _inspect_features(view: _PackageView) -> XlsxInspection:
    sheet = view.roots["xl/worksheets/sheet1.xml"]
    cells = sheet.findall(f".//{{{_MAIN_NS}}}c")
    formulas = [cell for cell in cells if cell.find(f"{{{_MAIN_NS}}}f") is not None]
    has_value = any(
        cell.find(f"{{{_MAIN_NS}}}v") is not None
        or cell.find(f"{{{_MAIN_NS}}}is") is not None
        for cell in cells
    )
    if not has_value or not formulas:
        raise XlsxConformanceError("XLSX value or formula cell is missing")
    admissions = {"values"}
    admissions.add("formulas")
    if all(cell.find(f"{{{_MAIN_NS}}}v") is not None for cell in formulas):
        admissions.add("cached-values")
    styled = any(cell.attrib.get("s") == "1" for cell in cells)
    conditional = sheet.find(f"{{{_MAIN_NS}}}conditionalFormatting") is not None
    if styled != conditional:
        raise XlsxConformanceError("XLSX style feature is incomplete")
    if styled:
        admissions.update(("styles", "conditional-formats"))
    printable = sheet.find(f"{{{_MAIN_NS}}}pageSetup") is not None
    merged = sheet.find(f"{{{_MAIN_NS}}}mergeCells") is not None
    sheet_count = len(
        view.roots["xl/workbook.xml"].findall(f".//{{{_MAIN_NS}}}sheet"),
    )
    if printable != merged or printable != (sheet_count == 2):
        raise XlsxConformanceError("XLSX print feature is incomplete")
    if printable:
        admissions.update(("print-settings", "merges", "sheets"))
    _inspect_graphics(view, admissions)
    text = view.parts["xl/worksheets/sheet1.xml"].decode()
    international = any(cell.attrib.get("s") == "2" for cell in cells)
    has_locale_format = b"[$-ja-JP]" in view.parts["xl/styles.xml"]
    has_locale_text = all(value in text for value in ("한글", "日本語", "العربية"))
    if international != has_locale_text or international != has_locale_format:
        raise XlsxConformanceError("XLSX international feature is incomplete")
    if international:
        admissions.add("international-formats")
    return XlsxInspection(view.names, frozenset(admissions), sheet_count)


def _inspect_graphics(
    view: _PackageView,
    admissions: set[str],
) -> None:
    graphical = "xl/drawings/drawing1.xml" in view.names
    if not graphical:
        return
    drawing = view.roots["xl/drawings/drawing1.xml"]
    checks = (
        drawing.find(f".//{{{_DRAWING_NS}}}sp") is not None,
        drawing.find(f".//{{{_DRAWING_NS}}}pic") is not None,
        "xl/charts/chart1.xml" in view.roots,
        view.parts.get("xl/media/image1.png", b"").startswith(b"\x89PNG\r\n\x1a\n"),
    )
    if not all(checks):
        raise XlsxConformanceError("XLSX graphical feature is incomplete")
    admissions.update(("drawings", "shapes", "images", "charts"))
