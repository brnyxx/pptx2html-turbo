from __future__ import annotations

from dataclasses import dataclass
from enum import StrEnum


class NegativeKind(StrEnum):
    TOKEN_ABSENT = "token_absent"


class SchemaExpectation(StrEnum):
    POSITIVE = "positive"
    NEGATIVE = "negative"


@dataclass(frozen=True, slots=True)
class NegativeSpec:
    kind: NegativeKind
    part: str
    token: str


@dataclass(frozen=True, slots=True)
class FeatureSpec:
    task: int
    deck: str
    feature_id: str
    part: str
    token: str
    negative: NegativeSpec | None = None
    schema_expectation: SchemaExpectation = SchemaExpectation.POSITIVE
    expected_diagnostic: str | None = None
    relationship_disposition: str = "none"


S = "ppt/slides/slide1.xml"
SR = "ppt/slides/_rels/slide1.xml.rels"


def _f(
    task: int,
    deck: str,
    feature_id: str,
    token: str,
    part: str = S,
    negative: NegativeSpec | None = None,
    schema_expectation: SchemaExpectation = SchemaExpectation.POSITIVE,
    expected_diagnostic: str | None = None,
    relationship_disposition: str = "none",
) -> FeatureSpec:
    return FeatureSpec(
        task,
        deck,
        feature_id,
        part,
        token,
        negative,
        schema_expectation,
        expected_diagnostic,
        relationship_disposition,
    )


ABSENT_REL = NegativeSpec(NegativeKind.TOKEN_ABSENT, SR, "rIdMissing")
ABSENT_TABLE_STYLE = NegativeSpec(
    NegativeKind.TOKEN_ABSENT,
    "ppt/tableStyles.xml",
    "{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}",
)
ABSENT_AUTHOR = NegativeSpec(
    NegativeKind.TOKEN_ABSENT,
    "ppt/commentAuthors.xml",
    'id="404"',
)


FEATURES = (
    _f(
        24,
        "thumbnail",
        "thumbnail",
        "thumbnail",
        part="_rels/.rels",
        relationship_disposition="internal",
    ),
    _f(
        24,
        "custom-xml",
        "custom-xml",
        "<demo:project",
        part="customXml/item1.xml",
    ),
    _f(
        24,
        "additional-characteristics",
        "additional-characteristics",
        "<ac:AdditionalCharacteristics",
        part="ppt/additionalCharacteristics.xml",
    ),
    _f(
        24,
        "bibliography",
        "bibliography",
        "<b:Sources",
        part="ppt/bibliography/sources.xml",
    ),
    _f(
        24,
        "extensions",
        "extensions",
        '<p:ext uri="{DEMO-EXTENSION}">',
        part="ppt/presentation.xml",
    ),
    _f(
        24,
        "handout-master",
        "handout-master",
        "<p:handoutMaster",
        part="ppt/handoutMasters/handoutMaster1.xml",
        relationship_disposition="internal",
    ),
    _f(24, "rtl-text", "rtl-text", '<a:pPr rtl="1"/>'),
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
        schema_expectation=SchemaExpectation.NEGATIVE,
        expected_diagnostic="DRAWINGML_PATTERN_UNSUPPORTED",
    ),
    _f(
        13, "picture-bullets", "picture-bullet-embedded", '<a:blip r:embed="rIdImage"/>'
    ),
    _f(
        13,
        "picture-bullets",
        "picture-bullet-missing",
        "<a:buBlip><a:blip/></a:buBlip>",
        negative=ABSENT_REL,
    ),
    _f(
        14,
        "table-styles",
        "table-style-regions",
        '<a:tblStyle styleId="{11111111-1111-1111-1111-111111111111}"',
        "ppt/tableStyles.xml",
    ),
    _f(
        14,
        "table-styles",
        "table-style-missing",
        '<a:tblPr firstCol="1" bandCol="1"><a:tableStyleId>{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}',
        negative=ABSENT_TABLE_STYLE,
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
        15,
        "actions",
        "action-table-frame",
        '<p:cNvPr id="17" name="action table">',
    ),
    _f(
        15,
        "actions",
        "action-group",
        '<p:cNvPr id="18" name="outer action group">',
    ),
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
        negative=ABSENT_AUTHOR,
    ),
    _f(17, "reflection-3d", "reflection", "<a:reflection "),
    _f(17, "reflection-3d", "drawingml-3d-fallback", "<a:scene3d>"),
    _f(18, "media", "media-audio", '<a:audioFile r:link="rIdAudio"/>'),
    _f(
        18,
        "media",
        "media-video",
        '<a:videoFile r:link="rIdVideo"/>',
        relationship_disposition="internal-video",
    ),
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
    _f(20, "charts", "chart-placeholder", "<c:stockChart>", "ppt/charts/chart3.xml"),
    _f(21, "fallback-domains", "fallback-smartart", "<dgm:relIds "),
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
