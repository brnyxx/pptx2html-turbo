import unittest
import zipfile
from typing import Final
from xml.etree import ElementTree


PRESENTATION: Final = "application/vnd.openxmlformats-officedocument.presentationml."
DRAWING: Final = "application/vnd.openxmlformats-officedocument.drawingml."
EXACT_TYPES: Final = {
    "ppt/presentation.xml": PRESENTATION + "presentation.main+xml",
    "ppt/presProps.xml": PRESENTATION + "presProps+xml",
    "ppt/slideMasters/slideMaster1.xml": PRESENTATION + "slideMaster+xml",
    "ppt/slideLayouts/slideLayout1.xml": PRESENTATION + "slideLayout+xml",
    "ppt/theme/theme1.xml": "application/vnd.openxmlformats-officedocument.theme+xml",
    "ppt/tableStyles.xml": PRESENTATION + "tableStyles+xml",
    "ppt/notesSlides/notesSlide1.xml": PRESENTATION + "notesSlide+xml",
    "ppt/notesMasters/notesMaster1.xml": PRESENTATION + "notesMaster+xml",
    "ppt/handoutMasters/handoutMaster1.xml": PRESENTATION + "handoutMaster+xml",
    "ppt/bibliography/sources.xml": "application/xml",
    "ppt/additionalCharacteristics.xml": "application/xml",
    "ppt/theme/notesTheme1.xml": "application/vnd.openxmlformats-officedocument.theme+xml",
    "ppt/comments/comment1.xml": PRESENTATION + "comments+xml",
    "ppt/commentAuthors.xml": PRESENTATION + "commentAuthors+xml",
    "ppt/comments/modernComment1.xml": "application/vnd.ms-powerpoint.comments+xml",
    "ppt/authors/author1.xml": "application/vnd.ms-powerpoint.authors+xml",
    "ppt/diagrams/data1.xml": DRAWING + "diagramData+xml",
    "ppt/diagrams/layout1.xml": DRAWING + "diagramLayout+xml",
    "ppt/diagrams/quickStyle1.xml": DRAWING + "diagramStyle+xml",
    "ppt/diagrams/colors1.xml": DRAWING + "diagramColors+xml",
    "ppt/embeddings/inert.bin": "application/vnd.openxmlformats-officedocument.oleObject",
}


def assert_content_types(case: unittest.TestCase, archive: zipfile.ZipFile) -> None:
    root = ElementTree.fromstring(archive.read("[Content_Types].xml"))
    namespace = {"ct": "http://schemas.openxmlformats.org/package/2006/content-types"}
    defaults = {
        row.get("Extension", ""): row.get("ContentType", "")
        for row in root.findall("ct:Default", namespace)
    }
    overrides = {
        row.get("PartName", "").lstrip("/"): row.get("ContentType", "")
        for row in root.findall("ct:Override", namespace)
    }
    for name in archive.namelist():
        if name == "[Content_Types].xml" or name.endswith(".rels"):
            continue
        actual = overrides.get(name, defaults.get(name.rsplit(".", 1)[-1]))
        case.assertEqual(actual, _expected(name), name)


def _expected(name: str) -> str:
    if name in EXACT_TYPES:
        return EXACT_TYPES[name]
    if name.startswith("ppt/slides/slide") and name.endswith(".xml"):
        return PRESENTATION + "slide+xml"
    if name.startswith("ppt/charts/chart") and name.endswith(".xml"):
        return DRAWING + "chart+xml"
    if name.endswith(".png"):
        return "image/png"
    if name.endswith(".svg"):
        return "image/svg+xml"
    if name.endswith(".wav"):
        return "audio/wav"
    if name.endswith(".mp4"):
        return "video/mp4"
    if name.endswith(".bin"):
        return "application/octet-stream"
    return "application/xml"
