from __future__ import annotations

from typing import Final

from evaluate.tests.completion_deck_feature_contract import REL


COMMON_RELS: Final = (
    ("_rels/.rels", "rId1", REL + "officeDocument", "ppt/presentation.xml", None),
    (
        "ppt/_rels/presentation.xml.rels",
        "rIdMaster",
        REL + "slideMaster",
        "slideMasters/slideMaster1.xml",
        None,
    ),
    (
        "ppt/_rels/presentation.xml.rels",
        "rIdPresProps",
        REL + "presProps",
        "presProps.xml",
        None,
    ),
    (
        "ppt/slideMasters/_rels/slideMaster1.xml.rels",
        "rIdLayout",
        REL + "slideLayout",
        "../slideLayouts/slideLayout1.xml",
        None,
    ),
    (
        "ppt/slideMasters/_rels/slideMaster1.xml.rels",
        "rIdTheme",
        REL + "theme",
        "../theme/theme1.xml",
        None,
    ),
    (
        "ppt/slideLayouts/_rels/slideLayout1.xml.rels",
        "rIdMaster",
        REL + "slideMaster",
        "../slideMasters/slideMaster1.xml",
        None,
    ),
)
