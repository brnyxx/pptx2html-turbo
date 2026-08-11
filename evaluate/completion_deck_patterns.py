from __future__ import annotations

from typing import Final
from xml.sax.saxutils import escape, quoteattr


PATTERN_PRESETS: Final = (
    "pct5",
    "pct10",
    "pct20",
    "pct25",
    "pct30",
    "pct40",
    "pct50",
    "pct60",
    "pct70",
    "pct75",
    "pct80",
    "pct90",
    "horz",
    "vert",
    "ltHorz",
    "ltVert",
    "dkHorz",
    "dkVert",
    "narHorz",
    "narVert",
    "dashHorz",
    "dashVert",
    "cross",
    "dnDiag",
    "upDiag",
    "ltDnDiag",
    "ltUpDiag",
    "dkDnDiag",
    "dkUpDiag",
    "wdDnDiag",
    "wdUpDiag",
    "dashDnDiag",
    "dashUpDiag",
    "diagCross",
    "smCheck",
    "lgCheck",
    "smGrid",
    "lgGrid",
    "dotGrid",
    "smConfetti",
    "lgConfetti",
    "horzBrick",
    "diagBrick",
    "solidDmnd",
    "openDmnd",
    "dotDmnd",
    "plaid",
    "sphere",
    "weave",
    "divot",
    "shingle",
    "wave",
    "trellis",
    "zigZag",
)
COLORS: Final = (
    '<a:fgClr><a:srgbClr val="4472C4"/></a:fgClr>'
    '<a:bgClr><a:srgbClr val="F2F2F2"/></a:bgClr>'
)


def pattern_slides(adjustment_shapes: str) -> tuple[tuple[str, str], ...]:
    groups = tuple(
        PATTERN_PRESETS[index : index + 18]
        for index in range(0, len(PATTERN_PRESETS), 18)
    )
    slides: list[tuple[str, str]] = []
    for slide_index, presets in enumerate(groups):
        shapes = "".join(
            _shape(shape_index + 2, preset)
            for shape_index, preset in enumerate(presets)
        )
        if slide_index == 0:
            shapes += _unknown_shape(30) + adjustment_shapes
        if slide_index == 2:
            shapes += _table(31)
        slides.append((shapes, ""))
    return tuple(slides)


def pattern_backgrounds() -> tuple[str, ...]:
    return (
        "",
        f'<p:bg><p:bgPr><a:pattFill prst="trellis">{COLORS}</a:pattFill></p:bgPr></p:bg>',
        "",
    )


def _shape(shape_id: int, preset: str) -> str:
    column = (shape_id - 2) % 6
    row = (shape_id - 2) // 6
    x = 180000 + column * 1450000
    y = 180000 + row * 2050000
    return (
        f"<p:sp><p:nvSpPr><p:cNvPr id={quoteattr(str(shape_id))} "
        f"name={quoteattr(f'pattern-{preset}')}/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr>"
        f'<a:xfrm><a:off x="{x}" y="{y}"/><a:ext cx="1250000" cy="1450000"/></a:xfrm>'
        f'<a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:pattFill prst={quoteattr(preset)}>'
        f"{COLORS}</a:pattFill></p:spPr><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r>"
        f'<a:rPr sz="1200"/><a:t>{escape(preset)}</a:t></a:r></a:p></p:txBody></p:sp>'
    )


def _unknown_shape(shape_id: int) -> str:
    return (
        f'<p:sp><p:nvSpPr><p:cNvPr id="{shape_id}" name="unknown pattern"/>'
        '<p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="7600000" y="5800000"/>'
        '<a:ext cx="1200000" cy="600000"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom>'
        '<a:pattFill prst="unknownFuturePattern"><a:fgClr><a:sysClr val="windowText" '
        'lastClr="112233"><a:shade val="50000"/></a:sysClr></a:fgClr></a:pattFill></p:spPr></p:sp>'
    )


def _table(shape_id: int) -> str:
    return (
        f'<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="{shape_id}" name="pattern-table"/>'
        '<p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm><a:off x="7000000" y="5500000"/>'
        '<a:ext cx="1800000" cy="800000"/></p:xfrm><a:graphic><a:graphicData '
        'uri="http://schemas.openxmlformats.org/drawingml/2006/table"><a:tbl><a:tblPr/><a:tblGrid>'
        '<a:gridCol w="1800000"/></a:tblGrid><a:tr h="800000"><a:tc><a:txBody><a:bodyPr/>'
        "<a:p><a:r><a:t>table pattern</a:t></a:r></a:p></a:txBody><a:tcPr>"
        f'<a:pattFill prst="diagCross">{COLORS}</a:pattFill></a:tcPr></a:tc></a:tr></a:tbl>'
        "</a:graphicData></a:graphic></p:graphicFrame>"
    )
