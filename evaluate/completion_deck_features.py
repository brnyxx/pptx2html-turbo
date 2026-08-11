from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True, slots=True)
class FeatureSpec:
    task: int
    deck: str
    feature_id: str
    part: str
    token: str


S = "ppt/slides/slide1.xml"
SR = "ppt/slides/_rels/slide1.xml.rels"


def _f(task: int, deck: str, feature_id: str, token: str, part: str = S) -> FeatureSpec:
    return FeatureSpec(task, deck, feature_id, part, token)


FEATURES = (
    _f(8, "patterns", "adjustment-basic", '<a:prstGeom prst="roundRect"><a:avLst>'),
    _f(9, "patterns", "adjustment-arrows", '<a:prstGeom prst="rightArrow"><a:avLst>'),
    _f(10, "patterns", "adjustment-remaining", '<a:prstGeom prst="wave"><a:avLst>'),
    _f(
        10,
        "patterns",
        "custom-geometry-unknown-formula",
        '<a:gd name="unknownGuide" fmla="unknownOp 1 2"/>',
    ),
    _f(12, "patterns", "pattern-fill-known", '<a:pattFill prst="pct5">'),
    _f(
        12,
        "patterns",
        "pattern-fill-unknown",
        '<a:pattFill prst="unknownFuturePattern">',
    ),
    _f(
        13, "picture-bullets", "picture-bullet-embedded", '<a:blip r:embed="rIdImage"/>'
    ),
    _f(
        13,
        "picture-bullets",
        "picture-bullet-missing",
        "<a:buBlip><a:blip/></a:buBlip>",
    ),
    _f(
        14,
        "table-styles",
        "table-style-regions",
        "{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}",
    ),
    _f(
        14,
        "table-styles",
        "table-style-missing",
        "{22222222-2222-2222-2222-222222222222}",
    ),
    _f(15, "actions", "action-external", '<Relationship Id="rIdExternal"', SR),
    _f(
        15,
        "actions",
        "action-internal",
        '<a:hlinkClick action="ppaction://hlinkshowjump?jump=nextslide"/>',
    ),
    _f(15, "actions", "action-unsafe", '<Relationship Id="rIdUnsafe"', SR),
    _f(
        16,
        "notes-comments",
        "notes-slide",
        "<p:notes ",
        "ppt/notesSlides/notesSlide1.xml",
    ),
    _f(
        16,
        "notes-comments",
        "comments-legacy",
        "<p:text>LEGACY_COMMENT</p:text>",
        "ppt/comments/comment1.xml",
    ),
    _f(
        16,
        "notes-comments",
        "comments-modern",
        "<p188:cm id=",
        "ppt/comments/modernComment1.xml",
    ),
    _f(
        16,
        "notes-comments",
        "comment-author-missing",
        '<p:cm authorId="404"',
        "ppt/comments/comment1.xml",
    ),
    _f(17, "reflection-3d", "reflection", "<a:reflection "),
    _f(17, "reflection-3d", "drawingml-3d-fallback", "<a:scene3d>"),
    _f(18, "media", "media-audio", '<a:audioFile r:link="rIdAudio"/>'),
    _f(18, "media", "media-video", '<a:videoFile r:link="rIdVideo"/>'),
    _f(18, "media", "media-unsupported", '<a:audioFile r:link="rIdUnsupported"/>'),
    _f(
        19,
        "timing-transitions",
        "transition-cut",
        '<p:transition spd="slow"><p:cut/>',
        "ppt/slides/slide2.xml",
    ),
    _f(
        19,
        "timing-transitions",
        "transition-fade",
        '<p:transition spd="slow"><p:fade/>',
    ),
    _f(
        19,
        "timing-transitions",
        "animation-bounded",
        '<p:animEffect transition="in" filter="fade">',
    ),
    _f(
        19,
        "timing-transitions",
        "animation-unsupported",
        '<p:animMotion origin="layout"',
    ),
    _f(20, "charts", "chart-direct", '<c:chart r:id="rIdChartDirect"/>'),
    _f(
        20,
        "charts",
        "chart-preview-fallback",
        '<Relationship Id="rIdPreviewImage"',
        "ppt/charts/_rels/chart2.xml.rels",
    ),
    _f(20, "charts", "chart-placeholder", "<c:stockChart/>", "ppt/charts/chart3.xml"),
    _f(21, "fallback-domains", "fallback-smartart", "<a:relIds "),
    _f(21, "fallback-domains", "fallback-ole", '<p:oleObj r:id="rIdOle"'),
    _f(
        21,
        "fallback-domains",
        "fallback-math",
        '<m:oMath xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math">',
    ),
    _f(21, "fallback-domains", "fallback-alternate-content", "<mc:AlternateContent>"),
    _f(
        21,
        "fallback-domains",
        "fallback-unknown-extension",
        '<unknown:payload xmlns:unknown="urn:pptx2html:test:unknown"',
    ),
)
