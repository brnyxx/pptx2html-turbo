from __future__ import annotations

import unittest
from dataclasses import dataclass
from enum import StrEnum
from typing import Final


S: Final = "ppt/slides/slide1.xml"
SR: Final = "ppt/slides/_rels/slide1.xml.rels"


class AbsenceKind(StrEnum):
    TOKEN_ABSENT = "token_absent"


@dataclass(frozen=True, slots=True)
class AbsenceRule:
    kind: AbsenceKind
    part: str
    token: str


@dataclass(frozen=True, slots=True)
class LocatorRule:
    part: str
    token: str
    negative: AbsenceRule | None = None


def _l(token: str, part: str = S, negative: AbsenceRule | None = None) -> LocatorRule:
    return LocatorRule(part, token, negative)


ABSENT_REL: Final = AbsenceRule(AbsenceKind.TOKEN_ABSENT, SR, "rIdMissing")
ABSENT_TABLE_STYLE: Final = AbsenceRule(
    AbsenceKind.TOKEN_ABSENT,
    "ppt/tableStyles.xml",
    "{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}",
)
ABSENT_AUTHOR: Final = AbsenceRule(
    AbsenceKind.TOKEN_ABSENT, "ppt/commentAuthors.xml", 'id="404"'
)


LOCATORS: Final = {
    "adjustment-basic": _l('<a:prstGeom prst="roundRect"><a:avLst>'),
    "adjustment-arrows": _l('<a:prstGeom prst="rightArrow"><a:avLst>'),
    "adjustment-remaining": _l('<a:prstGeom prst="wave"><a:avLst>'),
    "custom-geometry-unknown-formula": _l(
        '<a:gd name="unknownGuide" fmla="unknownOp 1 2"/>'
    ),
    "pattern-fill-known": _l('<a:pattFill prst="pct5">'),
    "pattern-fill-unknown": _l('<a:pattFill prst="unknownFuturePattern">'),
    "picture-bullet-embedded": _l('<a:blip r:embed="rIdImage"/>'),
    "picture-bullet-missing": _l("<a:buBlip><a:blip/></a:buBlip>", negative=ABSENT_REL),
    "table-style-regions": _l(
        '<a:tblStyle styleId="{11111111-1111-1111-1111-111111111111}"',
        "ppt/tableStyles.xml",
    ),
    "table-style-missing": _l(
        '<a:tblPr firstCol="1" bandCol="1"><a:tableStyleId>{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}',
        negative=ABSENT_TABLE_STYLE,
    ),
    "action-external": _l('<Relationship Id="rIdExternal"', SR),
    "action-internal": _l(
        '<a:hlinkClick action="ppaction://hlinkshowjump?jump=nextslide"/>'
    ),
    "action-unsafe": _l('<Relationship Id="rIdUnsafe"', SR),
    "action-table-frame": _l('<p:cNvPr id="17" name="action table">'),
    "action-group": _l('<p:cNvPr id="18" name="outer action group">'),
    "notes-slide": _l("<p:notes ", "ppt/notesSlides/notesSlide1.xml"),
    "comments-legacy": _l(
        "<p:text>LEGACY_COMMENT</p:text>", "ppt/comments/comment1.xml"
    ),
    "comments-modern": _l("<p188:cm id=", "ppt/comments/modernComment1.xml"),
    "comment-author-missing": _l(
        '<p:cm authorId="404"', "ppt/comments/comment1.xml", ABSENT_AUTHOR
    ),
    "reflection": _l("<a:reflection "),
    "drawingml-3d-fallback": _l("<a:scene3d>"),
    "media-audio": _l('<a:audioFile r:link="rIdAudio"/>'),
    "media-video": _l('<a:videoFile r:link="rIdVideo"/>'),
    "media-unsupported": _l('<a:audioFile r:link="rIdUnsupported"/>'),
    "transition-cut": _l('<p:transition spd="slow"><p:cut/>', "ppt/slides/slide2.xml"),
    "transition-fade": _l('<p:transition spd="slow"><p:fade/>'),
    "animation-bounded": _l('<p:animEffect transition="in" filter="fade">'),
    "animation-unsupported": _l('<p:animMotion origin="layout"'),
    "chart-direct": _l('<c:chart r:id="rIdChartDirect"/>'),
    "chart-preview-fallback": _l(
        '<Relationship Id="rIdPreviewImage"', "ppt/charts/_rels/chart2.xml.rels"
    ),
    "chart-placeholder": _l("<c:stockChart>", "ppt/charts/chart3.xml"),
    "fallback-smartart": _l("<dgm:relIds "),
    "fallback-ole": _l('<p:oleObj r:id="rIdOle"'),
    "fallback-math": _l(
        '<m:oMath xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math">'
    ),
    "fallback-alternate-content": _l("<mc:AlternateContent>"),
    "fallback-unknown-extension": _l(
        '<unknown:payload xmlns:unknown="urn:pptx2html:test:unknown"'
    ),
}


def assert_manifest_locators(
    case: unittest.TestCase, feature_rows: dict[str, object]
) -> None:
    case.assertEqual(set(feature_rows), set(LOCATORS))
    for feature_id, expected in LOCATORS.items():
        row = feature_rows[feature_id]
        case.assertIsInstance(row, dict)
        stimulus = row["stimulus"]
        case.assertEqual(stimulus["part"], expected.part, feature_id)
        case.assertEqual(stimulus["token"], expected.token, feature_id)
        negative = stimulus.get("negative")
        if expected.negative is None:
            case.assertIsNone(negative, feature_id)
            continue
        case.assertEqual(
            negative,
            {
                "kind": expected.negative.kind,
                "part": expected.negative.part,
                "token": expected.negative.token,
            },
            feature_id,
        )
