from __future__ import annotations

from typing import Final

if __package__:
    from .completion_deck_package import (
        NS,
        REL,
        Deck,
        png_bytes,
        relationships_xml,
        wav_bytes,
    )
else:
    from completion_deck_package import (
        NS,
        REL,
        Deck,
        png_bytes,
        relationships_xml,
        wav_bytes,
    )


PATTERNS: Final = '<p:sp><p:nvSpPr><p:cNvPr id="2" name="known pattern"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:pattFill prst="pct5"><a:fgClr><a:srgbClr val="336699"/></a:fgClr><a:bgClr><a:srgbClr val="F2F2F2"/></a:bgClr></a:pattFill></p:spPr></p:sp><p:sp><p:nvSpPr><p:cNvPr id="3" name="unknown pattern"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:pattFill prst="unknownFuturePattern"><a:fgClr><a:srgbClr val="112233"/></a:fgClr><a:bgClr><a:srgbClr val="FFFFFF"/></a:bgClr></a:pattFill></p:spPr></p:sp><p:sp><p:nvSpPr><p:cNvPr id="4" name="unknown custom formula"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:custGeom><a:avLst><a:gd name="unknownGuide" fmla="unknownOp 1 2"/></a:avLst><a:gdLst/><a:ahLst/><a:cxnLst/><a:rect l="l" t="t" r="r" b="b"/><a:pathLst><a:path w="100000" h="100000"><a:moveTo><a:pt x="0" y="0"/></a:moveTo><a:lnTo><a:pt x="100000" y="100000"/></a:lnTo></a:path></a:pathLst></a:custGeom></p:spPr></p:sp>'
PICTURE_BULLETS: Final = '<p:sp><p:nvSpPr><p:cNvPr id="2" name="picture bullets"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:pPr><a:buBlip><a:blip r:embed="rIdImage"/></a:buBlip></a:pPr><a:r><a:t>Present</a:t></a:r></a:p><a:p><a:pPr><a:buBlip><a:blip/></a:buBlip></a:pPr><a:r><a:t>Missing</a:t></a:r></a:p></p:txBody></p:sp>'
TABLES: Final = '<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="2" name="present style"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table"><a:tbl><a:tblPr firstRow="1" bandRow="1"><a:tableStyleId>{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}</a:tableStyleId></a:tblPr><a:tblGrid><a:gridCol w="3000000"/></a:tblGrid><a:tr h="500000"><a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p/></a:txBody><a:tcPr/></a:tc></a:tr></a:tbl></a:graphicData></a:graphic></p:graphicFrame><p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="3" name="missing style"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table"><a:tbl><a:tblPr><a:tableStyleId>{22222222-2222-2222-2222-222222222222}</a:tableStyleId></a:tblPr><a:tblGrid><a:gridCol w="3000000"/></a:tblGrid><a:tr h="500000"><a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p/></a:txBody><a:tcPr/></a:tc></a:tr></a:tbl></a:graphicData></a:graphic></p:graphicFrame>'
ACTIONS: Final = '<p:sp><p:nvSpPr><p:cNvPr id="2" name="internal"><a:hlinkClick action="ppaction://hlinkshowjump?jump=nextslide"/></p:cNvPr><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/></p:sp><p:sp><p:nvSpPr><p:cNvPr id="3" name="external"><a:hlinkClick r:id="rIdExternal"/></p:cNvPr><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/></p:sp><p:sp><p:nvSpPr><p:cNvPr id="4" name="unsafe"><a:hlinkClick r:id="rIdUnsafe"/></p:cNvPr><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/></p:sp>'
REFLECTION: Final = '<p:sp><p:nvSpPr><p:cNvPr id="2" name="reflection 3d"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:prstGeom prst="roundRect"><a:avLst/></a:prstGeom><a:effectLst><a:reflection blurRad="40000" stA="50000"/></a:effectLst><a:scene3d><a:camera prst="perspectiveFront"/><a:lightRig rig="threePt" dir="t"/></a:scene3d><a:sp3d extrusionH="120000" prstMaterial="warmMatte"/></p:spPr></p:sp>'
MEDIA: Final = '<p:pic><p:nvPicPr><p:cNvPr id="2" name="audio"><a:hlinkClick action="ppaction://media"/></p:cNvPr><p:cNvPicPr/><p:nvPr><a:audioFile r:link="rIdAudio"/></p:nvPr></p:nvPicPr><p:blipFill><a:blip r:embed="rIdPoster"/></p:blipFill><p:spPr/></p:pic><p:pic><p:nvPicPr><p:cNvPr id="3" name="video"/><p:cNvPicPr/><p:nvPr><a:videoFile r:link="rIdVideo"/></p:nvPr></p:nvPicPr><p:blipFill><a:blip r:embed="rIdPoster"/></p:blipFill><p:spPr/></p:pic><p:pic><p:nvPicPr><p:cNvPr id="4" name="unsupported"/><p:cNvPicPr/><p:nvPr><a:audioFile r:link="rIdUnsupported"/></p:nvPr></p:nvPicPr><p:blipFill><a:blip r:embed="rIdPoster"/></p:blipFill><p:spPr/></p:pic>'
TIMING_SHAPES: Final = '<p:sp><p:nvSpPr><p:cNvPr id="2" name="animated"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/></p:sp>'
TIMING_TAIL: Final = '<p:transition spd="slow"><p:{transition}/></p:transition><p:timing><p:tnLst><p:par><p:cTn id="1" dur="indefinite" nodeType="tmRoot"><p:childTnLst><p:animEffect transition="in" filter="fade"><p:cBhvr><p:cTn id="2" dur="500"/><p:tgtEl><p:spTgt spid="2"/></p:tgtEl></p:cBhvr></p:animEffect><p:animMotion origin="layout" path="M 0 0 L 1 1"><p:cBhvr><p:cTn id="3" dur="1000"/><p:tgtEl><p:spTgt spid="2"/></p:tgtEl></p:cBhvr></p:animMotion></p:childTnLst></p:cTn></p:par></p:tnLst></p:timing>'
CHARTS: Final = "".join(
    f'<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="{i}" name="{name}"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart"><c:chart r:id="{rid}"/></a:graphicData></a:graphic></p:graphicFrame>'
    for i, name, rid in (
        (2, "direct", "rIdChartDirect"),
        (3, "preview", "rIdChartPreview"),
        (4, "placeholder", "rIdChartPlaceholder"),
    )
)
FALLBACKS: Final = '<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="2" name="SmartArt"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/diagram"><a:relIds xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/diagram" r:dm="rIdDiagram" r:lo="" r:qs="" r:cs=""/></a:graphicData></a:graphic></p:graphicFrame><p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="3" name="OLE"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/presentationml/2006/ole"><p:oleObj r:id="rIdOle" progId="Package"/></a:graphicData></a:graphic></p:graphicFrame><mc:AlternateContent><mc:Choice Requires="x14"><p:sp><p:nvSpPr><p:cNvPr id="4" name="choice"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/></p:sp></mc:Choice><mc:Fallback><p:sp><p:nvSpPr><p:cNvPr id="5" name="fallback"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/></p:sp></mc:Fallback></mc:AlternateContent><p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="6" name="Office Math"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/officeDocument/2006/math"><m:oMath xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math"><m:r><m:t>x+1</m:t></m:r></m:oMath></a:graphicData></a:graphic></p:graphicFrame><p:extLst><p:ext uri="urn:pptx2html:test:unknown"><unknown:payload xmlns:unknown="urn:pptx2html:test:unknown"/></p:ext></p:extLst>'


def _comment_parts() -> tuple[tuple[str, bytes], ...]:
    return (
        (
            "ppt/notesSlides/notesSlide1.xml",
            f'<?xml version="1.0"?><p:notes {NS}><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/><p:sp><p:nvSpPr><p:cNvPr id="2" name="notes"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>NOTES_SENTINEL</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld></p:notes>'.encode(),
        ),
        (
            "ppt/comments/comment1.xml",
            f'<?xml version="1.0"?><p:cmLst {NS}><p:cm authorId="404" dt="2026-01-01T00:00:00Z" idx="1"><p:pos x="0" y="0"/><p:text>LEGACY_COMMENT</p:text></p:cm></p:cmLst>'.encode(),
        ),
        (
            "ppt/comments/modernComment1.xml",
            f'<?xml version="1.0"?><p188:cmLst {NS}><p188:cm id="{{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}}" authorId="{{AAAAAAAA-AAAA-AAAA-AAAA-AAAAAAAAAAAA}}" created="2026-01-01T00:00:00Z"><p188:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>MODERN_COMMENT</a:t></a:r></a:p></p188:txBody></p188:cm></p188:cmLst>'.encode(),
        ),
        (
            "ppt/authors/author1.xml",
            f'<?xml version="1.0"?><p188:authorLst {NS}><p188:author id="{{AAAAAAAA-AAAA-AAAA-AAAA-AAAAAAAAAAAA}}" name="Fixture" initials="F" userId="fixture@example.invalid" providerId=""/></p188:authorLst>'.encode(),
        ),
    )


def _chart_parts(image: bytes) -> tuple[tuple[str, bytes], ...]:
    return (
        (
            "ppt/charts/chart1.xml",
            b'<?xml version="1.0"?><c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"><c:chart><c:plotArea><c:barChart><c:barDir val="col"/><c:ser><c:idx val="0"/><c:order val="0"/></c:ser></c:barChart></c:plotArea></c:chart></c:chartSpace>',
        ),
        (
            "ppt/charts/chart2.xml",
            b'<?xml version="1.0"?><c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"><c:chart><c:plotArea><c:surface3DChart/></c:plotArea></c:chart></c:chartSpace>',
        ),
        (
            "ppt/charts/_rels/chart2.xml.rels",
            relationships_xml(
                (
                    (
                        "rIdPreviewImage",
                        REL + "image",
                        "../media/chart-preview.png",
                        None,
                    ),
                )
            ),
        ),
        (
            "ppt/charts/chart3.xml",
            b'<?xml version="1.0"?><c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"><c:chart><c:plotArea><c:stockChart/></c:plotArea></c:chart></c:chartSpace>',
        ),
        ("ppt/media/chart-preview.png", image),
    )


def build_decks(adjustment_shapes: str) -> tuple[Deck, ...]:
    image = png_bytes()
    comments_tail = '<p:extLst><p:ext uri="{6950BFC3-D8DA-4A85-94F7-54DA5524770B}"><p188:commentRel r:id="rIdModernComments"/></p:ext></p:extLst>'
    return (
        Deck("patterns", ((PATTERNS + adjustment_shapes, ""),)),
        Deck(
            "picture-bullets",
            ((PICTURE_BULLETS, ""),),
            (("rIdImage", REL + "image", "../media/bullet.png", None),),
            parts=(("ppt/media/bullet.png", image),),
        ),
        Deck(
            "table-styles",
            ((TABLES, ""),),
        ),
        Deck(
            "actions",
            ((ACTIONS, ""),),
            (
                ("rIdExternal", REL + "hyperlink", "https://example.com/", "External"),
                ("rIdUnsafe", REL + "hyperlink", "javascript:alert(1)", "External"),
            ),
        ),
        Deck(
            "notes-comments",
            (("", comments_tail),),
            (
                (
                    "rIdNotes",
                    REL + "notesSlide",
                    "../notesSlides/notesSlide1.xml",
                    None,
                ),
                ("rIdComments", REL + "comments", "../comments/comment1.xml", None),
                (
                    "rIdModernComments",
                    "http://schemas.microsoft.com/office/2018/10/relationships/comments",
                    "../comments/modernComment1.xml",
                    None,
                ),
            ),
            (
                (
                    "rIdAuthors",
                    "http://schemas.microsoft.com/office/2018/10/relationships/authors",
                    "authors/author1.xml",
                    None,
                ),
            ),
            _comment_parts(),
            (
                (
                    "/ppt/notesSlides/notesSlide1.xml",
                    "application/vnd.openxmlformats-officedocument.presentationml.notesSlide+xml",
                ),
                (
                    "/ppt/comments/comment1.xml",
                    "application/vnd.openxmlformats-officedocument.presentationml.comments+xml",
                ),
                (
                    "/ppt/comments/modernComment1.xml",
                    "application/vnd.ms-powerpoint.comments+xml",
                ),
                (
                    "/ppt/authors/author1.xml",
                    "application/vnd.ms-powerpoint.authors+xml",
                ),
            ),
        ),
        Deck("reflection-3d", ((REFLECTION, ""),)),
        Deck(
            "media",
            ((MEDIA, ""),),
            (
                ("rIdAudio", REL + "audio", "../media/audio.wav", None),
                ("rIdPoster", REL + "image", "../media/poster.png", None),
                (
                    "rIdVideo",
                    REL + "video",
                    "https://example.invalid/video.mp4",
                    "External",
                ),
                ("rIdUnsupported", REL + "audio", "../media/unsupported.bin", None),
            ),
            parts=(
                ("ppt/media/audio.wav", wav_bytes()),
                ("ppt/media/poster.png", image),
                ("ppt/media/unsupported.bin", b"INERT_UNSUPPORTED_MEDIA"),
            ),
        ),
        Deck(
            "timing-transitions",
            (
                (TIMING_SHAPES, TIMING_TAIL.format(transition="fade")),
                (TIMING_SHAPES, TIMING_TAIL.format(transition="cut")),
            ),
        ),
        Deck(
            "charts",
            ((CHARTS, ""),),
            (
                ("rIdChartDirect", REL + "chart", "../charts/chart1.xml", None),
                ("rIdChartPreview", REL + "chart", "../charts/chart2.xml", None),
                ("rIdChartPlaceholder", REL + "chart", "../charts/chart3.xml", None),
            ),
            parts=_chart_parts(image),
            types=tuple(
                (
                    f"/ppt/charts/chart{i}.xml",
                    "application/vnd.openxmlformats-officedocument.drawingml.chart+xml",
                )
                for i in range(1, 4)
            ),
        ),
        Deck(
            "fallback-domains",
            ((FALLBACKS, ""),),
            (
                ("rIdDiagram", REL + "diagramData", "../diagrams/data1.xml", None),
                ("rIdOle", REL + "oleObject", "../embeddings/inert.bin", None),
            ),
            parts=(
                (
                    "ppt/diagrams/data1.xml",
                    b'<?xml version="1.0"?><dgm:dataModel xmlns:dgm="http://schemas.openxmlformats.org/drawingml/2006/diagram"><dgm:ptLst/></dgm:dataModel>',
                ),
                ("ppt/embeddings/inert.bin", b"INERT_OLE_DO_NOT_EXECUTE"),
            ),
            types=(
                (
                    "/ppt/diagrams/data1.xml",
                    "application/vnd.openxmlformats-officedocument.drawingml.diagramData+xml",
                ),
                (
                    "/ppt/embeddings/inert.bin",
                    "application/vnd.openxmlformats-officedocument.oleObject",
                ),
            ),
        ),
    )
