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
        NS,
        REL,
        Deck,
        mp4_bytes,
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
        NS,
        REL,
        Deck,
        mp4_bytes,
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


def _action_shape(
    shape_id: int, name: str, action: str, label: str, x: int, y: int, preset: str = "rect"
) -> str:
    return f'<p:sp><p:nvSpPr><p:cNvPr id={quoteattr(str(shape_id))} name={quoteattr(name)}>{action}</p:cNvPr><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x={quoteattr(str(x))} y={quoteattr(str(y))}/><a:ext cx="1900000" cy="600000"/></a:xfrm><a:prstGeom prst={quoteattr(preset)}><a:avLst/></a:prstGeom></p:spPr><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr sz="1400"/><a:t>{label}</a:t></a:r></a:p></p:txBody></p:sp>'


ACTIONS: Final = "".join(
    (
        _action_shape(
            2,
            "next",
            '<a:hlinkClick action="ppaction://hlinkshowjump?jump=nextslide"/>',
            "NEXT_ACTION",
            200000,
            200000,
            "actionButtonForwardNext",
        ),
        _action_shape(
            3,
            "previous",
            '<a:hlinkClick action="ppaction://hlinkshowjump?jump=previousslide"/>',
            "PREVIOUS_ACTION",
            2300000,
            200000,
        ),
        _action_shape(
            4,
            "first",
            '<a:hlinkClick action="ppaction://hlinkshowjump?jump=firstslide"/>',
            "FIRST_ACTION",
            4400000,
            200000,
        ),
        _action_shape(
            5,
            "last",
            '<a:hlinkClick action="ppaction://hlinkshowjump?jump=lastslide"/>',
            "LAST_ACTION",
            6500000,
            200000,
        ),
        _action_shape(
            6,
            "specific",
            '<a:hlinkClick r:id="rIdSpecific" action="ppaction://hlinksldjump"/>',
            "SPECIFIC_ACTION",
            200000,
            1100000,
        ),
        _action_shape(
            7, "external", '<a:hlinkClick r:id="rIdExternal"/>', "HTTPS_ACTION", 2300000, 1100000
        ),
        _action_shape(
            8, "mailto", '<a:hlinkClick r:id="rIdMailto"/>', "MAILTO_ACTION", 4400000, 1100000
        ),
        _action_shape(
            9, "unsafe", '<a:hlinkClick r:id="rIdUnsafe"/>', "UNSAFE_VISIBLE", 6500000, 1100000
        ),
        _action_shape(
            10,
            "hover",
            '<a:hlinkMouseOver action="ppaction://hlinkshowjump?jump=lastslide"/>',
            "HOVER_ONLY",
            200000,
            2000000,
        ),
        _action_shape(
            11,
            "program",
            '<a:hlinkClick action="ppaction://program"/>',
            "PROGRAM_BLOCKED",
            2300000,
            2000000,
        ),
        _action_shape(
            12,
            "macro",
            '<a:hlinkClick action="ppaction://macro?name=SafeFixture"/>',
            "MACRO_BLOCKED",
            4400000,
            2000000,
        ),
        _action_shape(13, "no-op", "<a:hlinkClick/>", "NO_OP", 6500000, 2000000),
        '<p:pic><p:nvPicPr><p:cNvPr id="14" name="media"><a:hlinkClick r:id="" action="ppaction://media"/></p:cNvPr><p:cNvPicPr/><p:nvPr/></p:nvPicPr><p:blipFill/><p:spPr><a:xfrm><a:off x="200000" y="2900000"/><a:ext cx="1900000" cy="600000"/></a:xfrm></p:spPr></p:pic>',
        '<p:cxnSp><p:nvCxnSpPr><p:cNvPr id="15" name="connector"><a:hlinkClick action="ppaction://hlinkshowjump?jump=nextslide"/></p:cNvPr><p:cNvCxnSpPr/><p:nvPr/></p:nvCxnSpPr><p:spPr><a:xfrm><a:off x="2300000" y="2900000"/><a:ext cx="1900000" cy="600000"/></a:xfrm><a:prstGeom prst="line"><a:avLst/></a:prstGeom></p:spPr></p:cxnSp>',
        '<p:sp><p:nvSpPr><p:cNvPr id="16" name="run links"><a:hlinkClick r:id="rIdExternal"/></p:cNvPr><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="4400000" y="2900000"/><a:ext cx="4000000" cy="600000"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:noFill/></p:spPr><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr><a:hlinkClick r:id="rIdExternal"/></a:rPr><a:t>RUN_HTTPS</a:t></a:r><a:r><a:rPr/><a:t> | </a:t></a:r><a:r><a:rPr><a:hlinkClick r:id="rIdUnsafe"/></a:rPr><a:t>RUN_UNSAFE_VISIBLE</a:t></a:r></a:p></p:txBody></p:sp>',
        '<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="17" name="action table"><a:hlinkClick r:id="rIdExternal"/><a:hlinkMouseOver action="ppaction://program"/></p:cNvPr><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm><a:off x="200000" y="3800000"/><a:ext cx="4000000" cy="800000"/></p:xfrm><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table"><a:tbl><a:tblPr/><a:tblGrid><a:gridCol w="2000000"/></a:tblGrid><a:tr h="500000"><a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr><a:hlinkClick r:id="rIdMailto"/></a:rPr><a:t>TABLE_RUN_MAILTO</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc></a:tr></a:tbl></a:graphicData></a:graphic></p:graphicFrame>',
        '<p:grpSp><p:nvGrpSpPr><p:cNvPr id="18" name="outer action group"><a:hlinkClick r:id="rIdExternal"/><a:hlinkMouseOver action="ppaction://hlinkshowjump?jump=lastslide"/></p:cNvPr><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="4400000" y="3800000"/><a:ext cx="4000000" cy="1600000"/><a:chOff x="0" y="0"/><a:chExt cx="4000000" cy="1600000"/></a:xfrm></p:grpSpPr><p:grpSp><p:nvGrpSpPr><p:cNvPr id="19" name="inner action group"><a:hlinkClick action="ppaction://hlinkshowjump?jump=nextslide"/></p:cNvPr><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="3600000" cy="1200000"/><a:chOff x="0" y="0"/><a:chExt cx="3600000" cy="1200000"/></a:xfrm></p:grpSpPr><p:sp><p:nvSpPr><p:cNvPr id="20" name="group leaf"><a:hlinkClick action="ppaction://hlinkshowjump?jump=previousslide"/></p:cNvPr><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="1400000" y="300000"/><a:ext cx="1900000" cy="600000"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></p:spPr><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr sz="1400"/><a:t>GROUP_ACTION_LEAF</a:t></a:r></a:p></p:txBody></p:sp></p:grpSp></p:grpSp>',
    )
)


def _state_shape(label: str) -> str:
    return (
        '<p:sp><p:nvSpPr><p:cNvPr id="2" name="state label"/><p:cNvSpPr/><p:nvPr/>'
        '</p:nvSpPr><p:spPr><a:xfrm><a:off x="2200000" y="2600000"/>'
        '<a:ext cx="4800000" cy="1200000"/></a:xfrm><a:prstGeom prst="roundRect">'
        '<a:avLst/></a:prstGeom><a:solidFill><a:srgbClr val="DCEEFF"/></a:solidFill>'
        '</p:spPr><p:txBody><a:bodyPr anchor="ctr"/><a:lstStyle/><a:p><a:pPr algn="ctr"/>'
        f'<a:r><a:rPr sz="3200" b="1"/><a:t>{label}</a:t></a:r></a:p></p:txBody></p:sp>'
    )


REFLECTION: Final = (
    '<p:sp><p:nvSpPr><p:cNvPr id="2" name="reflection approximate"/>'
    '<p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm>'
    '<a:off x="914400" y="914400"/><a:ext cx="1828800" cy="914400"/>'
    '</a:xfrm><a:prstGeom prst="roundRect"><a:avLst/></a:prstGeom>'
    '<a:solidFill><a:srgbClr val="4472C4"/></a:solidFill><a:effectLst>'
    '<a:reflection blurRad="40000" stA="50000"/></a:effectLst></p:spPr>'
    '<p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r>'
    '<a:t>REFLECTION_APPROXIMATE_3D_FALLBACK</a:t></a:r></a:p></p:txBody></p:sp>'
    '<p:sp><p:nvSpPr><p:cNvPr id="3" name="ordered 3d fallback"/>'
    '<p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm>'
    '<a:off x="3657600" y="914400"/><a:ext cx="1828800" cy="914400"/>'
    '</a:xfrm><a:prstGeom prst="roundRect"><a:avLst/></a:prstGeom>'
    '<a:effectDag><a:cont name="first"><a:effectLst>'
    '<a:glow rad="12700"><a:srgbClr val="FF0000"/></a:glow></a:effectLst></a:cont>'
    '<a:cont name="second"><a:effectLst><a:outerShdw blurRad="12700" dist="12700"'
    ' dir="5400000"><a:srgbClr val="000000"/></a:outerShdw></a:effectLst></a:cont>'
    '</a:effectDag><a:scene3d><a:camera prst="perspectiveFront"/>'
    '<a:lightRig rig="threePt" dir="t"/></a:scene3d>'
    '<a:sp3d extrusionH="120000" prstMaterial="warmMatte"/></p:spPr></p:sp>'
)
MEDIA: Final = '<p:pic><p:nvPicPr><p:cNvPr id="2" name="audio"><a:hlinkClick action="ppaction://media"/></p:cNvPr><p:cNvPicPr/><p:nvPr><a:audioFile r:link="rIdAudio"/></p:nvPr></p:nvPicPr><p:blipFill><a:blip r:embed="rIdPoster"/></p:blipFill><p:spPr><a:xfrm><a:off x="200000" y="300000"/><a:ext cx="1800000" cy="1000000"/></a:xfrm></p:spPr></p:pic><p:pic><p:nvPicPr><p:cNvPr id="3" name="safe baseline AVC video"/><p:cNvPicPr/><p:nvPr><a:videoFile r:link="rIdVideo"/></p:nvPr></p:nvPicPr><p:blipFill><a:blip r:embed="rIdPoster"/></p:blipFill><p:spPr><a:xfrm><a:off x="2200000" y="300000"/><a:ext cx="1800000" cy="1300000"/></a:xfrm></p:spPr></p:pic><p:pic><p:nvPicPr><p:cNvPr id="4" name="unsupported codec"/><p:cNvPicPr/><p:nvPr><a:audioFile r:link="rIdUnsupported"/></p:nvPr></p:nvPicPr><p:blipFill><a:blip r:embed="rIdPoster"/></p:blipFill><p:spPr><a:xfrm><a:off x="4200000" y="300000"/><a:ext cx="1800000" cy="1000000"/></a:xfrm></p:spPr></p:pic><p:pic><p:nvPicPr><p:cNvPr id="5" name="external media never fetched"/><p:cNvPicPr/><p:nvPr><a:audioFile r:link="rIdExternalMedia"/></p:nvPr></p:nvPicPr><p:blipFill><a:blip r:embed="rIdPoster"/></p:blipFill><p:spPr><a:xfrm><a:off x="6200000" y="300000"/><a:ext cx="1800000" cy="1000000"/></a:xfrm></p:spPr></p:pic><p:pic><p:nvPicPr><p:cNvPr id="6" name="poster only"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr><p:blipFill><a:blip r:embed="rIdPoster"/></p:blipFill><p:spPr><a:xfrm><a:off x="200000" y="1800000"/><a:ext cx="1800000" cy="1000000"/></a:xfrm></p:spPr></p:pic>'
TIMING_SHAPES: Final = ''.join(
    f'<p:sp><p:nvSpPr><p:cNvPr id="{shape_id}" name="{name}">{action}</p:cNvPr><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="{x}" y="600000"/><a:ext cx="1800000" cy="900000"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:solidFill><a:srgbClr val="{color}"/></a:solidFill></p:spPr><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>{name}</a:t></a:r></a:p></p:txBody></p:sp>'
    for shape_id, name, x, color, action in (
        (2, "click group one", 400000, "4472C4", ""),
        (3, "with and after previous", 2600000, "70AD47", ""),
        (4, "unsupported stays visible", 4800000, "ED7D31", '<a:hlinkClick action="ppaction://hlinkshowjump?jump=nextslide"/>'),
    )
)
TIMING_TAIL: Final = '<p:transition spd="slow"><p:{transition}/></p:transition><p:timing><p:tnLst><p:par><p:cTn id="1" dur="indefinite" nodeType="tmRoot"><p:childTnLst><p:par><p:cTn id="10" nodeType="clickEffect"><p:stCondLst><p:cond delay="25"/></p:stCondLst><p:childTnLst><p:animEffect transition="in" filter="fade"><p:cBhvr><p:cTn id="11" dur="300"/><p:tgtEl><p:spTgt spid="2"/></p:tgtEl></p:cBhvr></p:animEffect></p:childTnLst></p:cTn></p:par><p:par><p:cTn id="12" nodeType="withEffect"><p:childTnLst><p:set><p:cBhvr><p:cTn id="13" dur="1"/><p:tgtEl><p:spTgt spid="3"/></p:tgtEl></p:cBhvr><p:to><p:strVal val="visible"/></p:to></p:set></p:childTnLst></p:cTn></p:par><p:par><p:cTn id="14" nodeType="afterEffect"><p:childTnLst><p:animEffect transition="out" filter="fade"><p:cBhvr><p:cTn id="15" dur="200"/><p:tgtEl><p:spTgt spid="3"/></p:tgtEl></p:cBhvr></p:animEffect></p:childTnLst></p:cTn></p:par><p:par><p:cTn id="20" nodeType="clickEffect"><p:childTnLst><p:animEffect transition="out" filter="fade"><p:cBhvr><p:cTn id="21" dur="250"/><p:tgtEl><p:spTgt spid="2"/></p:tgtEl></p:cBhvr></p:animEffect></p:childTnLst></p:cTn></p:par><p:animMotion origin="layout" path="M 0 0 L 1 1"><p:cBhvr><p:cTn id="30" dur="1000"/><p:tgtEl><p:spTgt spid="4"/></p:tgtEl></p:cBhvr></p:animMotion></p:childTnLst></p:cTn></p:par></p:tnLst></p:timing>'
CHARTS: Final = "".join(
    f'<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id={quoteattr(str(i))} name={quoteattr(name)}/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm><a:off x={quoteattr(str(x))} y="1200000"/><a:ext cx="2700000" cy="2400000"/></p:xfrm><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart"><c:chart r:id={quoteattr(rid)}/></a:graphicData></a:graphic></p:graphicFrame>'
    for i, name, rid, x in (
        (2, "direct", "rIdChartDirect", 200000),
        (3, "preview", "rIdChartPreview", 3200000),
        (4, "placeholder", "rIdChartPlaceholder", 6200000),
    )
)


def build_decks(adjustment_shapes: str) -> tuple[Deck, ...]:
    image = png_bytes()
    visible_notes_body = '<p:sp><p:nvSpPr><p:cNvPr id="2" name="visible body"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="914400" y="1828800"/><a:ext cx="7315200" cy="1371600"/></a:xfrm><a:prstGeom prst="roundRect"><a:avLst/></a:prstGeom><a:solidFill><a:srgbClr val="DCEEFF"/></a:solidFill></p:spPr><p:txBody><a:bodyPr anchor="ctr"/><a:lstStyle/><a:p><a:pPr algn="ctr"/><a:r><a:rPr sz="2800" b="1"/><a:t>VISIBLE_SLIDE_BODY</a:t></a:r></a:p></p:txBody></p:sp>'
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
            ((ACTIONS, ""), (_state_shape("STATE_SLIDE_2"), ""), (_state_shape("STATE_SLIDE_3"), "")),
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
            ((visible_notes_body, comments_tail),),
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
                ("rIdVideo", REL + "video", "../media/video.mp4", None),
                ("rIdUnsupported", REL + "audio", "../media/unsupported.bin", None),
                (
                    "rIdExternalMedia",
                    REL + "audio",
                    "https://media.invalid/never-fetch.wav",
                    "External",
                ),
            ),
            parts=(
                ("ppt/media/audio.wav", wav_bytes()),
                ("ppt/media/video.mp4", mp4_bytes()),
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
            "rtl-text",
            (
                (
                    '<p:sp><p:nvSpPr><p:cNvPr id="2" name="RTL mixed-script text"/>'
                    '<p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm>'
                    '<a:off x="914400" y="914400"/><a:ext cx="7315200" cy="1828800"/>'
                    '</a:xfrm><a:prstGeom prst="rect"/></p:spPr><p:txBody><a:bodyPr/>'
                    '<a:lstStyle/><a:p><a:pPr rtl="1"/><a:r><a:rPr lang="ar-SA"/>'
                    '<a:t>مرحبا PowerPoint 2026 بالعالم</a:t></a:r></a:p></p:txBody></p:sp>',
                    "",
                ),
            ),
        ),
        Deck(
            "handout-master",
            (("", ""),),
            presentation_rels=(
                (
                    "rIdHandout",
                    REL + "handoutMaster",
                    "handoutMasters/handoutMaster1.xml",
                    None,
                ),
            ),
            parts=(
                (
                    "ppt/handoutMasters/handoutMaster1.xml",
                    (
                        f'<?xml version="1.0"?><p:handoutMaster {NS}><p:cSld '
                        'name="Printed handout"><p:spTree><p:nvGrpSpPr/><p:grpSpPr/>'
                        '<p:sp><p:nvSpPr><p:cNvPr id="2" name="Header"/></p:nvSpPr>'
                        '<p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r>'
                        '<a:t>HANDOUT_HEADER</a:t></a:r></a:p></p:txBody></p:sp>'
                        '</p:spTree></p:cSld></p:handoutMaster>'
                    ).encode(),
                ),
            ),
            types=(
                (
                    "/ppt/handoutMasters/handoutMaster1.xml",
                    "application/vnd.openxmlformats-officedocument.presentationml.handoutMaster+xml",
                ),
            ),
        ),
        Deck(
            "extensions",
            (("", ""),),
            presentation_tail=(
                '<p:extLst><p:ext uri="{DEMO-EXTENSION}">'
                '<demo:payload xmlns:demo="urn:pptx2html:demo" enabled="1">'
                'EXTENSION_SENTINEL</demo:payload></p:ext></p:extLst>'
            ),
        ),
        Deck(
            "bibliography",
            (("", ""),),
            parts=(
                (
                    "ppt/bibliography/sources.xml",
                    (
                        '<?xml version="1.0"?><b:Sources '
                        'xmlns:b="http://schemas.openxmlformats.org/officeDocument/2006/bibliography">'
                        '<b:Source><b:Tag>Doe2026</b:Tag>'
                        '<b:SourceType>JournalArticle</b:SourceType>'
                        '<b:Title>Deterministic PPTX Conversion</b:Title><b:Year>2026</b:Year>'
                        '<b:Author><b:Author><b:NameList><b:Person>'
                        '<b:Last>Doe</b:Last><b:First>Jane</b:First>'
                        '</b:Person></b:NameList></b:Author></b:Author>'
                        '</b:Source></b:Sources>'
                    ).encode(),
                ),
            ),
            types=(("/ppt/bibliography/sources.xml", "application/xml"),),
        ),
        Deck(
            "additional-characteristics",
            (("", ""),),
            parts=(
                (
                    "ppt/additionalCharacteristics.xml",
                    (
                        '<?xml version="1.0"?><ac:AdditionalCharacteristics '
                        'xmlns:ac="http://schemas.openxmlformats.org/officeDocument/2006/additionalCharacteristics">'
                        '<ac:Characteristic name="supports3D" relation="ge" val="1" '
                        'vocabulary="urn:pptx2html:capabilities"/>'
                        '<ac:Characteristic name="rendererVersion" relation="eq" val="1.1.0"/>'
                        '</ac:AdditionalCharacteristics>'
                    ).encode(),
                ),
            ),
            types=(("/ppt/additionalCharacteristics.xml", "application/xml"),),
        ),
        Deck(
            "custom-xml",
            (("", ""),),
            parts=(
                (
                    "customXml/item1.xml",
                    (
                        '<?xml version="1.0"?><demo:project '
                        'xmlns:demo="urn:pptx2html:custom-data" id="alpha">'
                        '<demo:title>CUSTOM_XML_SENTINEL</demo:title></demo:project>'
                    ).encode(),
                ),
                (
                    "customXml/itemProps1.xml",
                    (
                        '<?xml version="1.0"?><ds:datastoreItem '
                        'xmlns:ds="http://schemas.openxmlformats.org/officeDocument/2006/customXml" '
                        'ds:itemID="{11111111-2222-3333-4444-555555555555}">'
                        '<ds:schemaRefs><ds:schemaRef ds:uri="urn:pptx2html:custom-data"/>'
                        '</ds:schemaRefs></ds:datastoreItem>'
                    ).encode(),
                ),
            ),
            types=(
                ("/customXml/item1.xml", "application/xml"),
                (
                    "/customXml/itemProps1.xml",
                    "application/vnd.openxmlformats-officedocument.customXmlProperties+xml",
                ),
            ),
        ),
        Deck(
            "thumbnail",
            (("", ""),),
            root_rels=(
                (
                    "rIdThumb",
                    "http://schemas.openxmlformats.org/package/2006/relationships/metadata/thumbnail",
                    "docProps/thumbnail.png",
                    None,
                ),
            ),
            parts=(("docProps/thumbnail.png", png_bytes()),),
            types=(("/docProps/thumbnail.png", "image/png"),),
        ),
        Deck(
            "theme-override",
            (("", ""),),
            layout_rels=(
                (
                    "rIdThemeOverride",
                    REL + "themeOverride",
                    "../theme/themeOverride1.xml",
                    None,
                ),
            ),
            parts=(
                (
                    "ppt/theme/themeOverride1.xml",
                    (
                        f'<?xml version="1.0"?><a:themeOverride {NS}>'
                        '<a:clrScheme name="Layout Override">'
                        '<a:dk1><a:srgbClr val="101010"/></a:dk1>'
                        '<a:lt1><a:srgbClr val="F0F0F0"/></a:lt1>'
                        '<a:accent1><a:srgbClr val="FF0000"/></a:accent1>'
                        '</a:clrScheme><a:fontScheme name="Override Fonts">'
                        '<a:majorFont/><a:minorFont/></a:fontScheme></a:themeOverride>'
                    ).encode(),
                ),
            ),
            types=(
                (
                    "/ppt/theme/themeOverride1.xml",
                    "application/vnd.openxmlformats-officedocument.themeOverride+xml",
                ),
            ),
        ),
        Deck(
            "content-part",
            (('<p:contentPart r:id="rIdContent"/>', ""),),
            slide_rels=(
                (
                    "rIdContent",
                    REL + "customXml",
                    "../customXml/smil1.xml",
                    "Internal",
                ),
            ),
            parts=(
                (
                    "ppt/customXml/smil1.xml",
                    (
                        '<?xml version="1.0"?>'
                        '<smil xmlns="http://www.w3.org/2001/SMIL20/Language">'
                        '<body><par dur="indefinite"/></body></smil>'
                    ).encode(),
                ),
            ),
            types=(("/ppt/customXml/smil1.xml", "application/xml"),),
        ),
        Deck(
            "slide-synchronization",
            (("", ""),),
            slide_rels=(
                (
                    "rIdSync",
                    REL + "slideUpdateInfo",
                    "../slideUpdateInfo/slideUpdateInfo1.xml",
                    None,
                ),
            ),
            parts=(
                (
                    "ppt/slideUpdateInfo/slideUpdateInfo1.xml",
                    (
                        f'<?xml version="1.0"?><p:sldSyncPr {NS} '
                        'serverSldId="server-slide-42" '
                        'serverSldModifiedTime="2026-08-12T10:30:00Z" '
                        'clientInsertedTime="2026-08-12T10:31:00Z"/>'
                    ).encode(),
                ),
            ),
            types=(
                (
                    "/ppt/slideUpdateInfo/slideUpdateInfo1.xml",
                    "application/vnd.openxmlformats-officedocument.presentationml.slideUpdateInfo+xml",
                ),
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
