from __future__ import annotations

from typing import Final
from xml.sax.saxutils import quoteattr

if __package__:
    from .completion_deck_charts import parts as chart_parts
    from .completion_deck_fallbacks import (
        FALLBACKS,
        content_types as fallback_content_types,
        parts as fallback_parts,
        relationships as fallback_relationships,
    )
    from .completion_deck_notes import (
        content_types as notes_content_types,
        parts as notes_parts,
        presentation_relationships as notes_presentation_relationships,
        slide_relationships as notes_slide_relationships,
    )
    from .completion_deck_package import (
        REL,
        Deck,
        png_bytes,
        wav_bytes,
    )
    from .completion_deck_patterns import pattern_backgrounds, pattern_slides
    from .completion_deck_picture_bullets import (
        PICTURE_BULLETS,
        content_types as picture_bullet_content_types,
        parts as picture_bullet_parts,
        relationships as picture_bullet_relationships,
    )
    from .completion_deck_tables import (
        TABLES,
        content_types as table_content_types,
        parts as table_parts,
        presentation_relationships as table_presentation_relationships,
    )
else:
    from completion_deck_charts import parts as chart_parts
    from completion_deck_fallbacks import (
        FALLBACKS,
        content_types as fallback_content_types,
        parts as fallback_parts,
        relationships as fallback_relationships,
    )
    from completion_deck_notes import (
        content_types as notes_content_types,
        parts as notes_parts,
        presentation_relationships as notes_presentation_relationships,
        slide_relationships as notes_slide_relationships,
    )
    from completion_deck_package import (
        REL,
        Deck,
        png_bytes,
        wav_bytes,
    )
    from completion_deck_patterns import pattern_backgrounds, pattern_slides
    from completion_deck_picture_bullets import (
        PICTURE_BULLETS,
        content_types as picture_bullet_content_types,
        parts as picture_bullet_parts,
        relationships as picture_bullet_relationships,
    )
    from completion_deck_tables import (
        TABLES,
        content_types as table_content_types,
        parts as table_parts,
        presentation_relationships as table_presentation_relationships,
    )


PATTERNS: Final = '<p:sp><p:nvSpPr><p:cNvPr id="50" name="unknown custom formula"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:custGeom><a:avLst><a:gd name="unknownGuide" fmla="unknownOp 1 2"/></a:avLst><a:gdLst/><a:ahLst/><a:cxnLst/><a:rect l="l" t="t" r="r" b="b"/><a:pathLst><a:path w="100000" h="100000"><a:moveTo><a:pt x="0" y="0"/></a:moveTo><a:lnTo><a:pt x="100000" y="100000"/></a:lnTo></a:path></a:pathLst></a:custGeom></p:spPr></p:sp>'
PICTURE_BULLETS: Final = '<p:sp><p:nvSpPr><p:cNvPr id="2" name="picture bullets"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:pPr><a:buBlip><a:blip r:embed="rIdImage"/></a:buBlip></a:pPr><a:r><a:t>Present</a:t></a:r></a:p><a:p><a:pPr><a:buBlip><a:blip/></a:buBlip></a:pPr><a:r><a:t>Missing</a:t></a:r></a:p></p:txBody></p:sp>'


def _action_shape(
    shape_id: int, name: str, action: str, label: str, x: int, preset: str = "rect"
) -> str:
    return f'<p:sp><p:nvSpPr><p:cNvPr id={quoteattr(str(shape_id))} name={quoteattr(name)}>{action}</p:cNvPr><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x={quoteattr(str(x))} y="200000"/><a:ext cx="1200000" cy="500000"/></a:xfrm><a:prstGeom prst={quoteattr(preset)}><a:avLst/></a:prstGeom></p:spPr><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>{label}</a:t></a:r></a:p></p:txBody></p:sp>'


ACTIONS: Final = "".join(
    (
        _action_shape(
            2,
            "next",
            '<a:hlinkClick action="ppaction://hlinkshowjump?jump=nextslide"/>',
            "NEXT_ACTION",
            200000,
            "actionButtonForwardNext",
        ),
        _action_shape(
            3,
            "previous",
            '<a:hlinkClick action="ppaction://hlinkshowjump?jump=previousslide"/>',
            "PREVIOUS_ACTION",
            1500000,
        ),
        _action_shape(
            4,
            "first",
            '<a:hlinkClick action="ppaction://hlinkshowjump?jump=firstslide"/>',
            "FIRST_ACTION",
            2800000,
        ),
        _action_shape(
            5,
            "last",
            '<a:hlinkClick action="ppaction://hlinkshowjump?jump=lastslide"/>',
            "LAST_ACTION",
            4100000,
        ),
        _action_shape(
            6,
            "specific",
            '<a:hlinkClick r:id="rIdSpecific" action="ppaction://hlinksldjump"/>',
            "SPECIFIC_ACTION",
            5400000,
        ),
        _action_shape(
            7, "external", '<a:hlinkClick r:id="rIdExternal"/>', "HTTPS_ACTION", 6700000
        ),
        _action_shape(
            8, "mailto", '<a:hlinkClick r:id="rIdMailto"/>', "MAILTO_ACTION", 200000
        ),
        _action_shape(
            9, "unsafe", '<a:hlinkClick r:id="rIdUnsafe"/>', "UNSAFE_VISIBLE", 1500000
        ),
        _action_shape(
            10,
            "hover",
            '<a:hlinkMouseOver action="ppaction://hlinkshowjump?jump=lastslide"/>',
            "HOVER_ONLY",
            2800000,
        ),
        _action_shape(
            11,
            "program",
            '<a:hlinkClick action="ppaction://program"/>',
            "PROGRAM_BLOCKED",
            4100000,
        ),
        _action_shape(
            12,
            "macro",
            '<a:hlinkClick action="ppaction://macro?name=SafeFixture"/>',
            "MACRO_BLOCKED",
            5400000,
        ),
        _action_shape(13, "no-op", "<a:hlinkClick/>", "NO_OP", 6700000),
        '<p:pic><p:nvPicPr><p:cNvPr id="14" name="media"><a:hlinkClick r:id="" action="ppaction://media"/></p:cNvPr><p:cNvPicPr/><p:nvPr/></p:nvPicPr><p:blipFill/><p:spPr><a:xfrm><a:off x="200000" y="1600000"/><a:ext cx="1200000" cy="500000"/></a:xfrm></p:spPr></p:pic>',
        '<p:cxnSp><p:nvCxnSpPr><p:cNvPr id="15" name="connector"><a:hlinkClick action="ppaction://hlinkshowjump?jump=nextslide"/></p:cNvPr><p:cNvCxnSpPr/><p:nvPr/></p:nvCxnSpPr><p:spPr><a:xfrm><a:off x="1500000" y="1600000"/><a:ext cx="1200000" cy="500000"/></a:xfrm><a:prstGeom prst="line"><a:avLst/></a:prstGeom></p:spPr></p:cxnSp>',
        '<p:sp><p:nvSpPr><p:cNvPr id="16" name="run links"><a:hlinkClick r:id="rIdExternal"/></p:cNvPr><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr><a:hlinkClick r:id="rIdExternal"/></a:rPr><a:t>RUN_HTTPS</a:t></a:r><a:r><a:rPr><a:hlinkClick r:id="rIdUnsafe"/></a:rPr><a:t>RUN_UNSAFE_VISIBLE</a:t></a:r></a:p></p:txBody></p:sp>',
        '<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="17" name="table run"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table"><a:tbl><a:tblPr/><a:tblGrid><a:gridCol w="2000000"/></a:tblGrid><a:tr h="500000"><a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr><a:hlinkClick r:id="rIdExternal"/></a:rPr><a:t>TABLE_RUN_HTTPS</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc></a:tr></a:tbl></a:graphicData></a:graphic></p:graphicFrame>',
    )
)
REFLECTION: Final = '<p:sp><p:nvSpPr><p:cNvPr id="2" name="reflection 3d"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:prstGeom prst="roundRect"><a:avLst/></a:prstGeom><a:effectLst><a:reflection blurRad="40000" stA="50000"/></a:effectLst><a:scene3d><a:camera prst="perspectiveFront"/><a:lightRig rig="threePt" dir="t"/></a:scene3d><a:sp3d extrusionH="120000" prstMaterial="warmMatte"/></p:spPr></p:sp>'
MEDIA: Final = '<p:pic><p:nvPicPr><p:cNvPr id="2" name="audio"><a:hlinkClick action="ppaction://media"/></p:cNvPr><p:cNvPicPr/><p:nvPr><a:audioFile r:link="rIdAudio"/></p:nvPr></p:nvPicPr><p:blipFill><a:blip r:embed="rIdPoster"/></p:blipFill><p:spPr/></p:pic><p:pic><p:nvPicPr><p:cNvPr id="3" name="video"/><p:cNvPicPr/><p:nvPr><a:videoFile r:link="rIdVideo"/></p:nvPr></p:nvPicPr><p:blipFill><a:blip r:embed="rIdPoster"/></p:blipFill><p:spPr/></p:pic><p:pic><p:nvPicPr><p:cNvPr id="4" name="unsupported"/><p:cNvPicPr/><p:nvPr><a:audioFile r:link="rIdUnsupported"/></p:nvPr></p:nvPicPr><p:blipFill><a:blip r:embed="rIdPoster"/></p:blipFill><p:spPr/></p:pic>'
TIMING_SHAPES: Final = '<p:sp><p:nvSpPr><p:cNvPr id="2" name="animated"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/></p:sp>'
TIMING_TAIL: Final = '<p:transition spd="slow"><p:{transition}/></p:transition><p:timing><p:tnLst><p:par><p:cTn id="1" dur="indefinite" nodeType="tmRoot"><p:childTnLst><p:animEffect transition="in" filter="fade"><p:cBhvr><p:cTn id="2" dur="500"/><p:tgtEl><p:spTgt spid="2"/></p:tgtEl></p:cBhvr></p:animEffect><p:animMotion origin="layout" path="M 0 0 L 1 1"><p:cBhvr><p:cTn id="3" dur="1000"/><p:tgtEl><p:spTgt spid="2"/></p:tgtEl></p:cBhvr></p:animMotion></p:childTnLst></p:cTn></p:par></p:tnLst></p:timing>'
CHARTS: Final = "".join(
    f'<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id={quoteattr(str(i))} name={quoteattr(name)}/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart"><c:chart r:id={quoteattr(rid)}/></a:graphicData></a:graphic></p:graphicFrame>'
    for i, name, rid in (
        (2, "direct", "rIdChartDirect"),
        (3, "preview", "rIdChartPreview"),
        (4, "placeholder", "rIdChartPlaceholder"),
    )
)


def build_decks(adjustment_shapes: str) -> tuple[Deck, ...]:
    image = png_bytes()
    comments_tail = '<p:extLst><p:ext uri="{6950BFC3-D8DA-4A85-94F7-54DA5524770B}"><p188:commentRel r:id="rIdModernComments"/></p:ext></p:extLst>'
    return (
        Deck(
            "patterns",
            pattern_slides(PATTERNS + adjustment_shapes),
            backgrounds=pattern_backgrounds(),
        ),
        Deck(
            "picture-bullets",
            ((PICTURE_BULLETS, ""),),
            picture_bullet_relationships(),
            parts=picture_bullet_parts(image),
            types=picture_bullet_content_types(),
        ),
        Deck(
            "table-styles",
            ((TABLES, ""),),
            presentation_rels=table_presentation_relationships(),
            parts=table_parts(),
            types=table_content_types(),
        ),
        Deck(
            "actions",
            ((ACTIONS, ""), ("", ""), ("", "")),
            (
                ("rIdExternal", REL + "hyperlink", "https://example.com/", "External"),
                (
                    "rIdMailto",
                    REL + "hyperlink",
                    "mailto:fixture@example.com",
                    "External",
                ),
                ("rIdUnsafe", REL + "hyperlink", "javascript:alert(1)", "External"),
                ("rIdSpecific", REL + "slide", "slide7.xml", None),
            ),
            slide_part_names=("slide1.xml", "slide42.xml", "slide7.xml"),
        ),
        Deck(
            "notes-comments",
            (("", comments_tail),),
            notes_slide_relationships(),
            notes_presentation_relationships(),
            notes_parts(),
            notes_content_types(),
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
            parts=chart_parts(image),
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
            fallback_relationships(),
            parts=fallback_parts(),
            types=fallback_content_types(),
        ),
    )
