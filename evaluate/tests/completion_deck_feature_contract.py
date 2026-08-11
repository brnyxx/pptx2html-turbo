from __future__ import annotations

import unittest
import zipfile
from dataclasses import dataclass
from enum import StrEnum
from pathlib import Path
from typing import Final
from xml.etree import ElementTree


NS: Final = {
    "a": "http://schemas.openxmlformats.org/drawingml/2006/main",
    "c": "http://schemas.openxmlformats.org/drawingml/2006/chart",
    "dgm": "http://schemas.openxmlformats.org/drawingml/2006/diagram",
    "m": "http://schemas.openxmlformats.org/officeDocument/2006/math",
    "mc": "http://schemas.openxmlformats.org/markup-compatibility/2006",
    "p": "http://schemas.openxmlformats.org/presentationml/2006/main",
    "p188": "http://schemas.microsoft.com/office/powerpoint/2018/8/main",
    "pr": "http://schemas.openxmlformats.org/package/2006/relationships",
    "r": "http://schemas.openxmlformats.org/officeDocument/2006/relationships",
}
REL: Final = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/"
S: Final = "ppt/slides/slide1.xml"
SR: Final = "ppt/slides/_rels/slide1.xml.rels"


@dataclass(frozen=True, slots=True)
class FeatureRule:
    feature_id: str
    deck: str
    part: str
    xpath: str | None
    attributes: tuple[tuple[str, str], ...] = ()
    text: str | None = None
    negative: "NegativePredicate | None" = None


class NegativePredicate(StrEnum):
    PICTURE_RELATION_ABSENT = "picture_relation_absent"


def _r(
    feature_id: str,
    deck: str,
    xpath: str,
    *attributes: tuple[str, str],
    part: str = S,
    text: str | None = None,
) -> FeatureRule:
    return FeatureRule(feature_id, deck, part, xpath, attributes, text)


def _negative_r(
    feature_id: str, deck: str, predicate: NegativePredicate
) -> FeatureRule:
    return FeatureRule(feature_id, deck, S, None, negative=predicate)


RULES: Final = (
    _r("adjustment-basic", "patterns", ".//a:prstGeom[@prst='roundRect']/a:avLst/a:gd"),
    _r(
        "adjustment-arrows",
        "patterns",
        ".//a:prstGeom[@prst='rightArrow']/a:avLst/a:gd",
    ),
    _r("adjustment-remaining", "patterns", ".//a:prstGeom[@prst='wave']/a:avLst/a:gd"),
    _r(
        "custom-geometry-unknown-formula",
        "patterns",
        ".//a:custGeom/a:avLst/a:gd",
        ("name", "unknownGuide"),
        ("fmla", "unknownOp 1 2"),
    ),
    _r("pattern-fill-known", "patterns", ".//a:pattFill[@prst='pct5']"),
    _r(
        "pattern-fill-unknown",
        "patterns",
        ".//a:pattFill[@prst='unknownFuturePattern']",
    ),
    _r(
        "picture-bullet-embedded",
        "picture-bullets",
        ".//a:buBlip/a:blip",
        (f"{{{NS['r']}}}embed", "rIdImage"),
    ),
    _negative_r(
        "picture-bullet-missing",
        "picture-bullets",
        NegativePredicate.PICTURE_RELATION_ABSENT,
    ),
    _r(
        "table-style-regions",
        "table-styles",
        ".//a:tblStyle",
        ("styleId", "{11111111-1111-1111-1111-111111111111}"),
        part="ppt/tableStyles.xml",
    ),
    _r(
        "table-style-missing",
        "table-styles",
        ".//a:tblPr/a:tableStyleId",
        text="{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}",
    ),
    _r(
        "action-external",
        "actions",
        ".//pr:Relationship[@Id='rIdExternal']",
        ("Type", REL + "hyperlink"),
        ("TargetMode", "External"),
        part=SR,
    ),
    _r(
        "action-internal",
        "actions",
        ".//a:hlinkClick",
        ("action", "ppaction://hlinkshowjump?jump=nextslide"),
    ),
    _r(
        "action-unsafe",
        "actions",
        ".//pr:Relationship[@Id='rIdUnsafe']",
        ("Type", REL + "hyperlink"),
        ("TargetMode", "External"),
        part=SR,
    ),
    _r(
        "notes-slide",
        "notes-comments",
        ".//a:t",
        part="ppt/notesSlides/notesSlide1.xml",
        text="NOTES_SENTINEL",
    ),
    _r(
        "comments-legacy",
        "notes-comments",
        ".//p:text",
        part="ppt/comments/comment1.xml",
        text="LEGACY_COMMENT",
    ),
    _r(
        "comments-modern",
        "notes-comments",
        ".//p188:cm",
        part="ppt/comments/modernComment1.xml",
    ),
    _r(
        "comment-author-missing",
        "notes-comments",
        ".//p:cm",
        ("authorId", "404"),
        part="ppt/comments/comment1.xml",
    ),
    _r("reflection", "reflection-3d", ".//a:reflection"),
    _r("drawingml-3d-fallback", "reflection-3d", ".//a:scene3d"),
    _r("media-audio", "media", ".//a:audioFile", (f"{{{NS['r']}}}link", "rIdAudio")),
    _r("media-video", "media", ".//a:videoFile", (f"{{{NS['r']}}}link", "rIdVideo")),
    _r(
        "media-unsupported",
        "media",
        ".//a:audioFile",
        (f"{{{NS['r']}}}link", "rIdUnsupported"),
    ),
    _r(
        "transition-cut",
        "timing-transitions",
        "./p:transition/p:cut",
        part="ppt/slides/slide2.xml",
    ),
    _r("transition-fade", "timing-transitions", "./p:transition/p:fade"),
    _r(
        "animation-bounded",
        "timing-transitions",
        "./p:timing//p:animEffect",
        ("filter", "fade"),
    ),
    _r(
        "animation-unsupported",
        "timing-transitions",
        "./p:timing//p:animMotion",
        ("origin", "layout"),
    ),
    _r("chart-direct", "charts", ".//c:chart", (f"{{{NS['r']}}}id", "rIdChartDirect")),
    _r(
        "chart-preview-fallback",
        "charts",
        ".//pr:Relationship[@Id='rIdPreviewImage']",
        ("Type", REL + "image"),
        part="ppt/charts/_rels/chart2.xml.rels",
    ),
    _r("chart-placeholder", "charts", ".//c:stockChart", part="ppt/charts/chart3.xml"),
    _r("fallback-smartart", "fallback-domains", ".//dgm:relIds"),
    _r(
        "fallback-ole",
        "fallback-domains",
        ".//p:oleObj",
        (f"{{{NS['r']}}}id", "rIdOle"),
    ),
    _r("fallback-math", "fallback-domains", ".//m:oMath"),
    _r("fallback-alternate-content", "fallback-domains", ".//mc:AlternateContent"),
    _r(
        "fallback-unknown-extension",
        "fallback-domains",
        ".//{urn:pptx2html:test:unknown}payload",
    ),
)


def assert_feature_contract(case: unittest.TestCase, root: Path) -> None:
    for rule in RULES:
        with zipfile.ZipFile(root / f"{rule.deck}.pptx") as archive:
            case.assertIn(rule.part, archive.namelist(), rule.feature_id)
            xml = ElementTree.fromstring(archive.read(rule.part))
            if rule.negative is NegativePredicate.PICTURE_RELATION_ABSENT:
                _assert_picture_bullet_absence(case, archive, xml)
                continue
            case.assertIsNotNone(rule.xpath, rule.feature_id)
            matches = xml.findall(rule.xpath, NS)
            if rule.text is not None:
                matches = [element for element in matches if element.text == rule.text]
            matches = [
                element
                for element in matches
                if all(element.get(name) == value for name, value in rule.attributes)
            ]
            case.assertTrue(matches, rule.feature_id)
    _assert_negative_absences(case, root)


def _assert_picture_bullet_absence(
    case: unittest.TestCase, archive: zipfile.ZipFile, slide: ElementTree.Element
) -> None:
    blips = slide.findall(".//a:buBlip/a:blip", NS)
    missing = [
        blip
        for blip in blips
        if blip.get(f"{{{NS['r']}}}embed") is None
        and blip.get(f"{{{NS['r']}}}link") is None
    ]
    case.assertEqual(len(missing), 1)
    rels = archive.read(SR)
    case.assertNotIn(b"rIdMissing", rels)


def _assert_negative_absences(case: unittest.TestCase, root: Path) -> None:
    with zipfile.ZipFile(root / "table-styles.pptx") as archive:
        case.assertNotIn(
            b"{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}",
            archive.read("ppt/tableStyles.xml"),
        )
    with zipfile.ZipFile(root / "notes-comments.pptx") as archive:
        case.assertNotIn(b'id="404"', archive.read("ppt/commentAuthors.xml"))
