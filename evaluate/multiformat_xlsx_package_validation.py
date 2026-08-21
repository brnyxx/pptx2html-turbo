from __future__ import annotations

from xml.etree import ElementTree

_CONTENT_NS = "http://schemas.openxmlformats.org/package/2006/content-types"
_REL_NS = "http://schemas.openxmlformats.org/package/2006/relationships"
_OFFICE_REL = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
_MAIN_NS = "http://schemas.openxmlformats.org/spreadsheetml/2006/main"
_DRAWING_NS = "http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing"
_DRAWING_MAIN_NS = "http://schemas.openxmlformats.org/drawingml/2006/main"
_CHART_NS = "http://schemas.openxmlformats.org/drawingml/2006/chart"
_CORE_NS = "http://schemas.openxmlformats.org/package/2006/metadata/core-properties"
_EXTENDED_NS = (
    "http://schemas.openxmlformats.org/officeDocument/2006/extended-properties"
)
_REL_ID = f"{{{_OFFICE_REL}}}id"
_REL_EMBED = f"{{{_OFFICE_REL}}}embed"


class XlsxPackageContractError(ValueError):
    pass


def validate_package_contract(
    names: tuple[str, ...],
    roots: dict[str, ElementTree.Element],
) -> None:
    _validate_root_tags(names, roots)
    _validate_content_types(names, roots["[Content_Types].xml"])
    _validate_root_relationships(roots["_rels/.rels"])
    _validate_workbook_relationships(names, roots)
    _validate_drawing_relationships(names, roots)


def _validate_root_tags(
    names: tuple[str, ...],
    roots: dict[str, ElementTree.Element],
) -> None:
    expected = {
        "[Content_Types].xml": f"{{{_CONTENT_NS}}}Types",
        "_rels/.rels": f"{{{_REL_NS}}}Relationships",
        "docProps/core.xml": f"{{{_CORE_NS}}}coreProperties",
        "docProps/app.xml": f"{{{_EXTENDED_NS}}}Properties",
        "xl/workbook.xml": f"{{{_MAIN_NS}}}workbook",
        "xl/_rels/workbook.xml.rels": f"{{{_REL_NS}}}Relationships",
        "xl/styles.xml": f"{{{_MAIN_NS}}}styleSheet",
        "xl/worksheets/sheet1.xml": f"{{{_MAIN_NS}}}worksheet",
    }
    if "xl/worksheets/sheet2.xml" in names:
        expected["xl/worksheets/sheet2.xml"] = f"{{{_MAIN_NS}}}worksheet"
    if "xl/drawings/drawing1.xml" in names:
        expected.update(
            {
                "xl/worksheets/_rels/sheet1.xml.rels": (f"{{{_REL_NS}}}Relationships"),
                "xl/drawings/drawing1.xml": f"{{{_DRAWING_NS}}}wsDr",
                "xl/drawings/_rels/drawing1.xml.rels": (f"{{{_REL_NS}}}Relationships"),
                "xl/charts/chart1.xml": f"{{{_CHART_NS}}}chartSpace",
            }
        )
    if {name: roots[name].tag for name in expected} != expected:
        raise XlsxPackageContractError("XLSX XML root tags differ")


def _validate_content_types(
    names: tuple[str, ...],
    root: ElementTree.Element,
) -> None:
    graphical = "xl/drawings/drawing1.xml" in names
    expected_defaults = {
        "rels": "application/vnd.openxmlformats-package.relationships+xml",
        "xml": "application/xml",
    }
    if graphical:
        expected_defaults["png"] = "image/png"
    expected_overrides = {
        "/docProps/core.xml": "application/vnd.openxmlformats-package.core-properties+xml",
        "/docProps/app.xml": "application/vnd.openxmlformats-officedocument.extended-properties+xml",
        "/xl/workbook.xml": "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml",
        "/xl/styles.xml": "application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml",
        "/xl/worksheets/sheet1.xml": "application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml",
    }
    if "xl/worksheets/sheet2.xml" in names:
        expected_overrides["/xl/worksheets/sheet2.xml"] = (
            "application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"
        )
    if graphical:
        expected_overrides.update(
            {
                "/xl/drawings/drawing1.xml": "application/vnd.openxmlformats-officedocument.drawing+xml",
                "/xl/charts/chart1.xml": "application/vnd.openxmlformats-officedocument.drawingml.chart+xml",
            }
        )
    defaults = _attributes(
        root.findall(f"{{{_CONTENT_NS}}}Default"),
        "Extension",
        "ContentType",
    )
    overrides = _attributes(
        root.findall(f"{{{_CONTENT_NS}}}Override"),
        "PartName",
        "ContentType",
    )
    if defaults != expected_defaults or overrides != expected_overrides:
        raise XlsxPackageContractError("XLSX content types differ")


def _validate_root_relationships(root: ElementTree.Element) -> None:
    expected = {
        "rId1": (
            f"{_OFFICE_REL}/officeDocument",
            "xl/workbook.xml",
        ),
        "rId2": (
            "http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties",
            "docProps/core.xml",
        ),
        "rId3": (
            f"{_OFFICE_REL}/extended-properties",
            "docProps/app.xml",
        ),
    }
    _expect_relationships(root, expected)


def _validate_workbook_relationships(
    names: tuple[str, ...],
    roots: dict[str, ElementTree.Element],
) -> None:
    printable = "xl/worksheets/sheet2.xml" in names
    expected = {
        "rId1": (f"{_OFFICE_REL}/worksheet", "worksheets/sheet1.xml"),
        "rId2": (f"{_OFFICE_REL}/styles", "styles.xml"),
    }
    if printable:
        expected["rId3"] = (
            f"{_OFFICE_REL}/worksheet",
            "worksheets/sheet2.xml",
        )
    _expect_relationships(roots["xl/_rels/workbook.xml.rels"], expected)
    sheets = roots["xl/workbook.xml"].findall(f".//{{{_MAIN_NS}}}sheet")
    expected_ids = ["rId1", *(["rId3"] if printable else [])]
    if [sheet.attrib.get(_REL_ID) for sheet in sheets] != expected_ids:
        raise XlsxPackageContractError("XLSX workbook sheet bindings differ")


def _validate_drawing_relationships(
    names: tuple[str, ...],
    roots: dict[str, ElementTree.Element],
) -> None:
    graphical = "xl/drawings/drawing1.xml" in names
    drawing = roots["xl/worksheets/sheet1.xml"].find(f"{{{_MAIN_NS}}}drawing")
    if not graphical:
        if drawing is not None:
            raise XlsxPackageContractError("XLSX worksheet drawing binding differs")
        return
    if drawing is None or drawing.attrib.get(_REL_ID) != "rId1":
        raise XlsxPackageContractError("XLSX worksheet drawing binding differs")
    _expect_relationships(
        roots["xl/worksheets/_rels/sheet1.xml.rels"],
        {"rId1": (f"{_OFFICE_REL}/drawing", "../drawings/drawing1.xml")},
    )
    _expect_relationships(
        roots["xl/drawings/_rels/drawing1.xml.rels"],
        {
            "rId1": (f"{_OFFICE_REL}/chart", "../charts/chart1.xml"),
            "rId2": (f"{_OFFICE_REL}/image", "../media/image1.png"),
        },
    )
    drawing_root = roots["xl/drawings/drawing1.xml"]
    chart = drawing_root.find(f".//{{{_CHART_NS}}}chart")
    image = drawing_root.find(f".//{{{_DRAWING_MAIN_NS}}}blip")
    if (
        chart is None
        or chart.attrib.get(_REL_ID) != "rId1"
        or image is None
        or image.attrib.get(_REL_EMBED) != "rId2"
    ):
        raise XlsxPackageContractError("XLSX drawing part bindings differ")


def _expect_relationships(
    root: ElementTree.Element,
    expected: dict[str, tuple[str, str]],
) -> None:
    relationships = {
        item.attrib.get("Id", ""): (
            item.attrib.get("Type", ""),
            item.attrib.get("Target", ""),
        )
        for item in root.findall(f"{{{_REL_NS}}}Relationship")
    }
    if relationships != expected:
        raise XlsxPackageContractError("XLSX relationship contract differs")


def _attributes(
    elements: list[ElementTree.Element],
    key: str,
    value: str,
) -> dict[str, str]:
    result = {
        element.attrib.get(key, ""): element.attrib.get(value, "")
        for element in elements
    }
    if len(result) != len(elements):
        raise XlsxPackageContractError("XLSX package attributes are duplicated")
    return result
