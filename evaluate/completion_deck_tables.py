from __future__ import annotations

from xml.sax.saxutils import quoteattr

if __package__:
    from .completion_deck_package import REL, ContentType, Part, Relationship
else:
    from completion_deck_package import REL, ContentType, Part, Relationship


CUSTOM_STYLE = "{11111111-1111-1111-1111-111111111111}"
UNAVAILABLE_STYLE = "{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}"


def _cell(label: str, properties: str = "<a:tcPr/>", attributes: str = "") -> str:
    return f"<a:tc{attributes}><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>{label}</a:t></a:r></a:p></a:txBody>{properties}</a:tc>"


def _positive_table() -> str:
    rows: list[str] = []
    for row in range(4):
        cells: list[str] = []
        for column in range(4):
            properties = "<a:tcPr/>"
            if (row, column) == (1, 1):
                properties = '<a:tcPr><a:solidFill><a:srgbClr val="ABCDEF"/></a:solidFill></a:tcPr>'
            elif (row, column) == (1, 2):
                properties = "<a:tcPr><a:noFill/></a:tcPr>"
            cells.append(_cell(f"r{row}c{column}", properties))
        rows.append(f'<a:tr h="500000">{"".join(cells)}</a:tr>')
    merged = (
        _cell("merged", attributes=' gridSpan="2"')
        + _cell("merge-continuation", attributes=' hMerge="1"')
        + _cell("logical-last")
    )
    rows.append(f'<a:tr h="500000">{merged}</a:tr>')
    grid = "<a:tblGrid>" + '<a:gridCol w="1000000"/>' * 4 + "</a:tblGrid>"
    properties = f'<a:tblPr firstRow="1" lastRow="1" firstCol="1" lastCol="1" bandRow="1" bandCol="1"><a:tableStyleId>{CUSTOM_STYLE}</a:tableStyleId></a:tblPr>'
    return f'<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="2" name="present style"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table"><a:tbl>{properties}{grid}{"".join(rows)}</a:tbl></a:graphicData></a:graphic></p:graphicFrame>'


def _negative_table() -> str:
    return f'<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="3" name="missing style"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table"><a:tbl><a:tblPr firstCol="1" bandCol="1"><a:tableStyleId>{UNAVAILABLE_STYLE}</a:tableStyleId></a:tblPr><a:tblGrid><a:gridCol w="3000000"/></a:tblGrid><a:tr h="500000">{_cell("built-in unavailable")}</a:tr></a:tbl></a:graphicData></a:graphic></p:graphicFrame>'


TABLES = _positive_table() + _negative_table()


def presentation_relationships() -> tuple[Relationship, ...]:
    return (("rIdTableStyles", REL + "tableStyles", "tableStyles.xml", None),)


def parts() -> tuple[Part, ...]:
    return (("ppt/tableStyles.xml", _styles()),)


def content_types() -> tuple[ContentType, ...]:
    return (
        (
            "/ppt/tableStyles.xml",
            "application/vnd.openxmlformats-officedocument.presentationml.tableStyles+xml",
        ),
    )


def _region(name: str, color: str) -> str:
    encoded = quoteattr(color)
    return f'<a:{name}><a:tcTxStyle><a:fontRef idx="minor"><a:schemeClr val={encoded}/></a:fontRef><a:schemeClr val="tx1"/></a:tcTxStyle><a:tcStyle><a:tcBdr/><a:fill><a:solidFill><a:schemeClr val={encoded}/></a:solidFill></a:fill></a:tcStyle></a:{name}>'


def _styles() -> bytes:
    ordered = (
        ("wholeTbl", "accent1"),
        ("band1H", "accent2"),
        ("band2H", "accent3"),
        ("band1V", "accent4"),
        ("band2V", "accent5"),
        ("lastCol", "accent6"),
        ("firstCol", "accent1"),
        ("lastRow", "accent2"),
        ("seCell", "accent3"),
        ("swCell", "accent4"),
        ("firstRow", "accent5"),
        ("neCell", "accent6"),
        ("nwCell", "accent1"),
    )
    regions = "".join(_region(name, color) for name, color in ordered)
    style = quoteattr(CUSTOM_STYLE)
    return f'<?xml version="1.0"?><a:tblStyleLst xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" def={style}><a:tblStyle styleId={style} styleName="Completion Regions">{regions}</a:tblStyle></a:tblStyleLst>'.encode()
