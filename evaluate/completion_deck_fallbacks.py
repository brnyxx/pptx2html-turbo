from __future__ import annotations

if __package__:
    from .completion_deck_package import REL, ContentType, Part, Relationship
else:
    from completion_deck_package import REL, ContentType, Part, Relationship


DGM = "http://schemas.openxmlformats.org/drawingml/2006/diagram"
A = "http://schemas.openxmlformats.org/drawingml/2006/main"

FALLBACKS = '<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="2" name="SmartArt"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/diagram"><dgm:relIds xmlns:dgm="http://schemas.openxmlformats.org/drawingml/2006/diagram" r:dm="rIdDiagramData" r:lo="rIdDiagramLayout" r:qs="rIdDiagramStyle" r:cs="rIdDiagramColors"/></a:graphicData></a:graphic></p:graphicFrame><p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="3" name="OLE"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/presentationml/2006/ole"><p:oleObj r:id="rIdOle" progId="Package"><p:embed/></p:oleObj></a:graphicData></a:graphic></p:graphicFrame><mc:AlternateContent><mc:Choice Requires="x14" xmlns:x14="http://schemas.microsoft.com/office/drawing/2010/main"><p:sp><p:nvSpPr><p:cNvPr id="4" name="choice"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/></p:sp></mc:Choice><mc:Fallback><p:sp><p:nvSpPr><p:cNvPr id="5" name="fallback"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/></p:sp></mc:Fallback></mc:AlternateContent><p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="6" name="Office Math"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/officeDocument/2006/math"><m:oMath xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math"><m:r><m:t>x+1</m:t></m:r></m:oMath></a:graphicData></a:graphic></p:graphicFrame><p:extLst><p:ext uri="urn:pptx2html:test:unknown"><unknown:payload xmlns:unknown="urn:pptx2html:test:unknown"/></p:ext></p:extLst>'


def relationships() -> tuple[Relationship, ...]:
    return (
        ("rIdDiagramData", REL + "diagramData", "../diagrams/data1.xml", None),
        ("rIdDiagramLayout", REL + "diagramLayout", "../diagrams/layout1.xml", None),
        (
            "rIdDiagramStyle",
            REL + "diagramQuickStyle",
            "../diagrams/quickStyle1.xml",
            None,
        ),
        (
            "rIdDiagramColors",
            REL + "diagramColors",
            "../diagrams/colors1.xml",
            None,
        ),
        ("rIdOle", REL + "oleObject", "../embeddings/inert.bin", None),
    )


def parts() -> tuple[Part, ...]:
    return (
        ("ppt/diagrams/data1.xml", _data()),
        ("ppt/diagrams/layout1.xml", _layout()),
        ("ppt/diagrams/quickStyle1.xml", _style()),
        ("ppt/diagrams/colors1.xml", _colors()),
        ("ppt/embeddings/inert.bin", b"INERT_OLE_DO_NOT_EXECUTE"),
    )


def content_types() -> tuple[ContentType, ...]:
    drawing = "application/vnd.openxmlformats-officedocument.drawingml."
    return (
        ("/ppt/diagrams/data1.xml", drawing + "diagramData+xml"),
        ("/ppt/diagrams/layout1.xml", drawing + "diagramLayout+xml"),
        ("/ppt/diagrams/quickStyle1.xml", drawing + "diagramStyle+xml"),
        ("/ppt/diagrams/colors1.xml", drawing + "diagramColors+xml"),
        (
            "/ppt/embeddings/inert.bin",
            "application/vnd.openxmlformats-officedocument.oleObject",
        ),
    )


def _data() -> bytes:
    return f'<?xml version="1.0"?><dgm:dataModel xmlns:dgm="{DGM}" xmlns:a="{A}"><dgm:ptLst><dgm:pt modelId="0" type="doc"/></dgm:ptLst><dgm:cxnLst/></dgm:dataModel>'.encode()


def _layout() -> bytes:
    return f'<?xml version="1.0"?><dgm:layoutDef xmlns:dgm="{DGM}" xmlns:a="{A}" uniqueId="urn:pptx2html:layout" minVer="12.0"><dgm:title val="Completion"/><dgm:desc val="Completion"/><dgm:catLst/><dgm:layoutNode name="root"/></dgm:layoutDef>'.encode()


def _style() -> bytes:
    refs = '<a:lnRef idx="0"><a:schemeClr val="accent1"/></a:lnRef><a:fillRef idx="1"><a:schemeClr val="accent1"/></a:fillRef><a:effectRef idx="0"><a:schemeClr val="accent1"/></a:effectRef><a:fontRef idx="minor"><a:schemeClr val="tx1"/></a:fontRef>'
    return f'<?xml version="1.0"?><dgm:styleDef xmlns:dgm="{DGM}" xmlns:a="{A}" uniqueId="urn:pptx2html:style" minVer="12.0"><dgm:title val="Completion"/><dgm:desc val="Completion"/><dgm:catLst/><dgm:styleLbl name="node"><dgm:style>{refs}</dgm:style></dgm:styleLbl></dgm:styleDef>'.encode()


def _colors() -> bytes:
    colors = '<dgm:fillClrLst><a:schemeClr val="accent1"/></dgm:fillClrLst><dgm:linClrLst><a:schemeClr val="accent1"/></dgm:linClrLst><dgm:effectClrLst><a:schemeClr val="accent1"/></dgm:effectClrLst><dgm:txLinClrLst><a:schemeClr val="tx1"/></dgm:txLinClrLst><dgm:txFillClrLst><a:schemeClr val="tx1"/></dgm:txFillClrLst><dgm:txEffectClrLst><a:schemeClr val="tx1"/></dgm:txEffectClrLst>'
    return f'<?xml version="1.0"?><dgm:colorsDef xmlns:dgm="{DGM}" xmlns:a="{A}" uniqueId="urn:pptx2html:colors" minVer="12.0"><dgm:title val="Completion"/><dgm:desc val="Completion"/><dgm:catLst/><dgm:styleLbl name="node">{colors}</dgm:styleLbl></dgm:colorsDef>'.encode()
