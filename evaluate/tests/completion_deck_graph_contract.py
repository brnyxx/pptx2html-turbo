from __future__ import annotations

import posixpath
import unittest
import zipfile
from typing import Final
from xml.etree import ElementTree

from evaluate.tests.completion_deck_common_rel_contract import COMMON_RELS
from evaluate.tests.completion_deck_feature_contract import NS, REL, RULES, SR

SPECIFIC_RELS: Final = {
    "picture-bullets": ((SR, "rIdImage", REL + "image", "../media/bullet.png", None),),
    "actions": (
        (SR, "rIdExternal", REL + "hyperlink", "https://example.com/", "External"),
        (SR, "rIdUnsafe", REL + "hyperlink", "javascript:alert(1)", "External"),
    ),
    "notes-comments": (
        (SR, "rIdNotes", REL + "notesSlide", "../notesSlides/notesSlide1.xml", None),
        (SR, "rIdComments", REL + "comments", "../comments/comment1.xml", None),
        (
            SR,
            "rIdModernComments",
            "http://schemas.microsoft.com/office/2018/10/relationships/comments",
            "../comments/modernComment1.xml",
            None,
        ),
        (
            "ppt/_rels/presentation.xml.rels",
            "rIdModernAuthors",
            "http://schemas.microsoft.com/office/2018/10/relationships/authors",
            "authors/author1.xml",
            None,
        ),
        (
            "ppt/_rels/presentation.xml.rels",
            "rIdClassicAuthors",
            REL + "commentAuthors",
            "commentAuthors.xml",
            None,
        ),
        (
            "ppt/_rels/presentation.xml.rels",
            "rIdNotesMaster",
            REL + "notesMaster",
            "notesMasters/notesMaster1.xml",
            None,
        ),
        (
            "ppt/notesSlides/_rels/notesSlide1.xml.rels",
            "rIdSlide",
            REL + "slide",
            "../slides/slide1.xml",
            None,
        ),
        (
            "ppt/notesSlides/_rels/notesSlide1.xml.rels",
            "rIdNotesMaster",
            REL + "notesMaster",
            "../notesMasters/notesMaster1.xml",
            None,
        ),
        (
            "ppt/notesMasters/_rels/notesMaster1.xml.rels",
            "rIdTheme",
            REL + "theme",
            "../theme/notesTheme1.xml",
            None,
        ),
    ),
    "media": (
        (SR, "rIdAudio", REL + "audio", "../media/audio.wav", None),
        (SR, "rIdPoster", REL + "image", "../media/poster.png", None),
        (
            SR,
            "rIdVideo",
            REL + "video",
            "https://example.invalid/video.mp4",
            "External",
        ),
        (SR, "rIdUnsupported", REL + "audio", "../media/unsupported.bin", None),
    ),
    "charts": (
        (SR, "rIdChartDirect", REL + "chart", "../charts/chart1.xml", None),
        (SR, "rIdChartPreview", REL + "chart", "../charts/chart2.xml", None),
        (SR, "rIdChartPlaceholder", REL + "chart", "../charts/chart3.xml", None),
        (
            "ppt/charts/_rels/chart2.xml.rels",
            "rIdPreviewImage",
            REL + "image",
            "../media/chart-preview.png",
            None,
        ),
    ),
    "fallback-domains": (
        (SR, "rIdDiagramData", REL + "diagramData", "../diagrams/data1.xml", None),
        (
            SR,
            "rIdDiagramLayout",
            REL + "diagramLayout",
            "../diagrams/layout1.xml",
            None,
        ),
        (
            SR,
            "rIdDiagramStyle",
            REL + "diagramQuickStyle",
            "../diagrams/quickStyle1.xml",
            None,
        ),
        (
            SR,
            "rIdDiagramColors",
            REL + "diagramColors",
            "../diagrams/colors1.xml",
            None,
        ),
        (SR, "rIdOle", REL + "oleObject", "../embeddings/inert.bin", None),
    ),
    "table-styles": (
        (
            "ppt/_rels/presentation.xml.rels",
            "rIdTableStyles",
            REL + "tableStyles",
            "tableStyles.xml",
            None,
        ),
    ),
}


def assert_package_graph(
    case: unittest.TestCase, archive: zipfile.ZipFile, deck: str
) -> None:
    names = set(archive.namelist())
    adjacency: dict[str, set[str]] = {}
    for rel_part in (name for name in names if name.endswith(".rels")):
        source = (
            "" if rel_part == "_rels/.rels" else rel_part.replace("/_rels/", "/")[:-5]
        )
        if source:
            case.assertIn(source, names, rel_part)
        for rel in ElementTree.fromstring(archive.read(rel_part)).findall(
            "pr:Relationship", NS
        ):
            target, mode, kind = (
                rel.get("Target", ""),
                rel.get("TargetMode"),
                rel.get("Type", ""),
            )
            case.assertTrue(
                kind.startswith(REL)
                or kind.startswith(
                    "http://schemas.microsoft.com/office/2018/10/relationships/"
                ),
                rel_part,
            )
            if mode == "External":
                case.assertTrue(
                    target.startswith(("http://", "https://", "javascript:")), rel_part
                )
                continue
            case.assertIsNone(mode, rel_part)
            resolved = posixpath.normpath(
                posixpath.join(posixpath.dirname(source), target)
            )
            case.assertIn(resolved, names, f"{rel_part}:{rel.get('Id')}")
            adjacency.setdefault(source, set()).add(resolved)
    reachable, pending = {""}, [""]
    while pending:
        for target in adjacency.get(pending.pop(), set()) - reachable:
            reachable.add(target)
            pending.append(target)
    for rule in (item for item in RULES if item.deck == deck):
        feature_part = (
            rule.part.replace("/_rels/", "/")[:-5]
            if rule.part.endswith(".rels")
            else rule.part
        )
        case.assertIn(feature_part, reachable, rule.feature_id)
    _assert_relationship_expectations(case, archive, deck)


def _assert_relationship_expectations(
    case: unittest.TestCase, archive: zipfile.ZipFile, deck: str
) -> None:
    expected = (*COMMON_RELS, *SPECIFIC_RELS.get(deck, ()))
    for part, rid, kind, target, mode in expected:
        root = ElementTree.fromstring(archive.read(part))
        relation = root.find(f"pr:Relationship[@Id='{rid}']", NS)
        case.assertIsNotNone(relation, f"{deck}:{rid}")
        case.assertEqual(
            (relation.get("Type"), relation.get("Target"), relation.get("TargetMode")),
            (kind, target, mode),
            f"{deck}:{rid}",
        )
    for index in range(
        1,
        1
        + len(
            [
                name
                for name in archive.namelist()
                if name.startswith("ppt/slides/slide") and name.endswith(".xml")
            ]
        ),
    ):
        presentation = ElementTree.fromstring(
            archive.read("ppt/_rels/presentation.xml.rels")
        )
        slide_rel = presentation.find(f"pr:Relationship[@Id='rIdSlide{index}']", NS)
        case.assertEqual(
            (
                slide_rel.get("Type"),
                slide_rel.get("Target"),
                slide_rel.get("TargetMode"),
            ),
            (REL + "slide", f"slides/slide{index}.xml", None),
        )
        slide_rels = ElementTree.fromstring(
            archive.read(f"ppt/slides/_rels/slide{index}.xml.rels")
        )
        layout = slide_rels.find("pr:Relationship[@Id='rIdLayout']", NS)
        case.assertEqual(
            (layout.get("Type"), layout.get("Target"), layout.get("TargetMode")),
            (REL + "slideLayout", "../slideLayouts/slideLayout1.xml", None),
        )
