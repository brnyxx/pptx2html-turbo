from __future__ import annotations

from xml.sax.saxutils import quoteattr

if __package__:
    from .completion_deck_package import REL, ContentType, Part, Relationship
else:
    from completion_deck_package import REL, ContentType, Part, Relationship


CUSTOM_STYLE = "{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}"
UNKNOWN_STYLE = "{22222222-2222-2222-2222-222222222222}"


TABLES = f'<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="2" name="present style"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table"><a:tbl><a:tblPr firstRow="1" bandRow="1"><a:tableStyleId>{CUSTOM_STYLE}</a:tableStyleId></a:tblPr><a:tblGrid><a:gridCol w="3000000"/></a:tblGrid><a:tr h="500000"><a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p/></a:txBody><a:tcPr/></a:tc></a:tr></a:tbl></a:graphicData></a:graphic></p:graphicFrame><p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="3" name="missing style"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table"><a:tbl><a:tblPr><a:tableStyleId>{UNKNOWN_STYLE}</a:tableStyleId></a:tblPr><a:tblGrid><a:gridCol w="3000000"/></a:tblGrid><a:tr h="500000"><a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p/></a:txBody><a:tcPr/></a:tc></a:tr></a:tbl></a:graphicData></a:graphic></p:graphicFrame>'


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
