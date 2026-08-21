from __future__ import annotations

from html import escape

from evaluate.multiformat_xlsx_features import (
    FeatureFlags,
    XlsxCase,
    chart_xml,
    drawing_xml,
    seed_png,
    styles_xml,
    worksheet_xml,
)

REL_NS = "http://schemas.openxmlformats.org/package/2006/relationships"
OFFICE_REL = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"


def package_parts(
    case: XlsxCase,
    flags: FeatureFlags,
) -> tuple[tuple[str, bytes], ...]:
    parts = [
        ("[Content_Types].xml", _content_types(flags)),
        ("_rels/.rels", _root_relationships()),
        ("docProps/core.xml", _core_properties()),
        ("docProps/app.xml", _app_properties(flags)),
        ("xl/workbook.xml", _workbook(flags)),
        ("xl/_rels/workbook.xml.rels", _workbook_relationships(flags)),
        ("xl/styles.xml", styles_xml(flags)),
        ("xl/worksheets/sheet1.xml", worksheet_xml(case, flags)),
    ]
    if flags.graphical:
        parts.append(("xl/worksheets/_rels/sheet1.xml.rels", _sheet_relationships()))
    if flags.printable:
        parts.append(
            (
                "xl/worksheets/sheet2.xml",
                worksheet_xml(case, flags, secondary=True),
            ),
        )
    if flags.graphical:
        parts.extend(
            (
                ("xl/drawings/drawing1.xml", drawing_xml()),
                ("xl/drawings/_rels/drawing1.xml.rels", _drawing_relationships()),
                ("xl/charts/chart1.xml", chart_xml(case)),
                ("xl/media/image1.png", seed_png(case.feature_seed)),
            ),
        )
    return tuple(parts)


def _content_types(flags: FeatureFlags) -> bytes:
    overrides = [
        (
            "/docProps/core.xml",
            "application/vnd.openxmlformats-package.core-properties+xml",
        ),
        (
            "/docProps/app.xml",
            "application/vnd.openxmlformats-officedocument.extended-properties+xml",
        ),
        (
            "/xl/workbook.xml",
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml",
        ),
        (
            "/xl/styles.xml",
            "application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml",
        ),
        (
            "/xl/worksheets/sheet1.xml",
            "application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml",
        ),
    ]
    if flags.printable:
        overrides.append(
            (
                "/xl/worksheets/sheet2.xml",
                "application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml",
            ),
        )
    if flags.graphical:
        overrides.extend(
            (
                (
                    "/xl/drawings/drawing1.xml",
                    "application/vnd.openxmlformats-officedocument.drawing+xml",
                ),
                (
                    "/xl/charts/chart1.xml",
                    "application/vnd.openxmlformats-officedocument.drawingml.chart+xml",
                ),
            ),
        )
    values = "".join(
        f'<Override PartName="{part}" ContentType="{content_type}"/>'
        for part, content_type in overrides
    )
    png = (
        '<Default Extension="png" ContentType="image/png"/>' if flags.graphical else ""
    )
    return (
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
        '<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>'
        f'<Default Extension="xml" ContentType="application/xml"/>{png}{values}</Types>'
    ).encode()


def _root_relationships() -> bytes:
    return _relationships(
        (
            (
                "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument",
                "xl/workbook.xml",
            ),
            (
                "http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties",
                "docProps/core.xml",
            ),
            (
                "http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties",
                "docProps/app.xml",
            ),
        ),
    )


def _workbook_relationships(flags: FeatureFlags) -> bytes:
    values = [
        (f"{OFFICE_REL}/worksheet", "worksheets/sheet1.xml"),
        (f"{OFFICE_REL}/styles", "styles.xml"),
    ]
    if flags.printable:
        values.append((f"{OFFICE_REL}/worksheet", "worksheets/sheet2.xml"))
    return _relationships(tuple(values))


def _sheet_relationships() -> bytes:
    return _relationships(((f"{OFFICE_REL}/drawing", "../drawings/drawing1.xml"),))


def _drawing_relationships() -> bytes:
    return _relationships(
        (
            (f"{OFFICE_REL}/chart", "../charts/chart1.xml"),
            (f"{OFFICE_REL}/image", "../media/image1.png"),
        ),
    )


def _relationships(values: tuple[tuple[str, str], ...]) -> bytes:
    relationships = "".join(
        f'<Relationship Id="rId{index}" Type="{kind}" Target="{target}"/>'
        for index, (kind, target) in enumerate(values, start=1)
    )
    return (
        f'<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        f'<Relationships xmlns="{REL_NS}">{relationships}</Relationships>'
    ).encode()


def _workbook(flags: FeatureFlags) -> bytes:
    second = (
        '<sheet name="Print Area" sheetId="2" r:id="rId3"/>' if flags.printable else ""
    )
    defined = (
        '<definedNames><definedName name="_xlnm.Print_Area" localSheetId="0">'
        "'Conformance'!$A$1:$D$5</definedName></definedNames>"
        if flags.printable
        else ""
    )
    return (
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        '<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" '
        'xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">'
        f'<sheets><sheet name="Conformance" sheetId="1" r:id="rId1"/>{second}</sheets>'
        f'{defined}<calcPr calcId="191029" fullCalcOnLoad="0"/></workbook>'
    ).encode()


def _core_properties() -> bytes:
    return (
        b'<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        b'<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" '
        b'xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/" '
        b'xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><dc:creator>xlsx-conformance</dc:creator>'
        b'<cp:lastModifiedBy>xlsx-conformance</cp:lastModifiedBy><dcterms:created xsi:type="dcterms:W3CDTF">'
        b'2000-01-01T00:00:00Z</dcterms:created><dcterms:modified xsi:type="dcterms:W3CDTF">'
        b"2000-01-01T00:00:00Z</dcterms:modified></cp:coreProperties>"
    )


def _app_properties(flags: FeatureFlags) -> bytes:
    names = ("Conformance", "Print Area") if flags.printable else ("Conformance",)
    titles = "".join(f"<vt:lpstr>{escape(name)}</vt:lpstr>" for name in names)
    return (
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        '<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties" '
        'xmlns:vt="http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes">'
        f"<Application>Deterministic OOXML Builder</Application><Sheets>{len(names)}</Sheets>"
        f'<TitlesOfParts><vt:vector size="{len(names)}" baseType="lpstr">{titles}'
        "</vt:vector></TitlesOfParts></Properties>"
    ).encode()
