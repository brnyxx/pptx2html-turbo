from __future__ import annotations

import binascii
import struct
import zlib
from dataclasses import dataclass
from enum import StrEnum
from html import escape
from typing import assert_never


class XlsxStratum(StrEnum):
    VALUES_FORMULAS = "values-formulas"
    STYLES_CONDITIONAL_FORMATS = "styles-conditional-formats"
    PRINT_LAYOUT = "print-layout"
    CHARTS_IMAGES_SHAPES = "charts-images-shapes"
    INTERNATIONAL_FORMATS = "international-formats"
    MIXED_STRESS = "mixed-stress"


@dataclass(frozen=True, slots=True)
class XlsxCase:
    case_id: str
    ordinal: int
    stratum: XlsxStratum
    feature_seed: str


@dataclass(frozen=True, slots=True)
class FeatureFlags:
    styled: bool
    printable: bool
    graphical: bool
    international: bool

    @property
    def admissions(self) -> frozenset[str]:
        values = {"values", "formulas", "cached-values"}
        if self.styled:
            values.update(("styles", "conditional-formats"))
        if self.printable:
            values.update(("print-settings", "sheets", "merges"))
        if self.graphical:
            values.update(("charts", "drawings", "images", "shapes"))
        if self.international:
            values.add("international-formats")
        return frozenset(values)


def feature_flags(stratum: XlsxStratum) -> FeatureFlags:
    match stratum:
        case XlsxStratum.VALUES_FORMULAS:
            return FeatureFlags(False, False, False, False)
        case XlsxStratum.STYLES_CONDITIONAL_FORMATS:
            return FeatureFlags(True, False, False, False)
        case XlsxStratum.PRINT_LAYOUT:
            return FeatureFlags(False, True, False, False)
        case XlsxStratum.CHARTS_IMAGES_SHAPES:
            return FeatureFlags(False, False, True, False)
        case XlsxStratum.INTERNATIONAL_FORMATS:
            return FeatureFlags(False, False, False, True)
        case XlsxStratum.MIXED_STRESS:
            return FeatureFlags(True, True, True, True)
        case unreachable:
            assert_never(unreachable)


def worksheet_xml(
    case: XlsxCase,
    flags: FeatureFlags,
    *,
    secondary: bool = False,
) -> bytes:
    if secondary:
        rows = '<row r="1"><c r="A1" t="inlineStr"><is><t>Print Area</t></is></c></row>'
    else:
        style = ' s="1"' if flags.styled else ""
        international = (
            '<row r="5"><c r="A5" t="inlineStr"><is><t>한글 日本語 العربية</t></is></c>'
            '<c r="B5" s="2"><v>45292</v></c></row>'
            if flags.international
            else ""
        )
        merged = (
            '<row r="4"><c r="B4" t="inlineStr"><is><t>Merged layout</t></is></c></row>'
            if flags.printable
            else ""
        )
        rows = (
            f'<row r="1" ht="48" customHeight="1"><c r="A1" t="inlineStr">'
            f"<is><t>{escape(case.case_id)}</t></is></c>"
            f'<c r="B1" s="3" t="inlineStr"><is><t>{case.feature_seed}</t></is></c></row>'
            f'<row r="2"><c r="A2"{style}><v>{case.ordinal}</v></c></row>'
            f'<row r="3"><c r="A3"><f>SUM(A2,1)</f><v>{case.ordinal + 1}</v></c></row>'
            f"{merged}{international}"
        )
    merge = (
        '<mergeCells count="1"><mergeCell ref="B4:D4"/></mergeCells>'
        if flags.printable and not secondary
        else ""
    )
    conditional = (
        '<conditionalFormatting sqref="A2"><cfRule type="cellIs" dxfId="0" priority="1" '
        'operator="greaterThan"><formula>0</formula></cfRule></conditionalFormatting>'
        if flags.styled and not secondary
        else ""
    )
    printing = (
        '<printOptions horizontalCentered="1"/><pageMargins left="0.25" right="0.25" '
        'top="0.5" bottom="0.5" header="0.2" footer="0.2"/>'
        '<pageSetup orientation="landscape" fitToWidth="1" fitToHeight="1"/>'
        if flags.printable
        else ""
    )
    drawing = '<drawing r:id="rId1"/>' if flags.graphical and not secondary else ""
    return (
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        '<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" '
        'xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">'
        '<cols><col min="1" max="1" width="28" customWidth="1"/>'
        '<col min="2" max="2" width="24" customWidth="1"/></cols>'
        f"<sheetData>{rows}</sheetData>{merge}{conditional}{printing}{drawing}</worksheet>"
    ).encode()


def styles_xml(flags: FeatureFlags) -> bytes:
    custom_format = (
        '<numFmts count="1"><numFmt numFmtId="164" formatCode="[$-ja-JP]yyyy\\-mm\\-dd"/></numFmts>'
        if flags.international
        else ""
    )
    return (
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        '<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">'
        f'{custom_format}<fonts count="2"><font><sz val="11"/><name val="Liberation Sans"/></font>'
        '<font><b/><color rgb="FFFFFFFF"/><sz val="11"/><name val="Liberation Sans"/></font></fonts>'
        '<fills count="3"><fill><patternFill patternType="none"/></fill><fill><patternFill '
        'patternType="gray125"/></fill><fill><patternFill patternType="solid"><fgColor '
        'rgb="FF305496"/><bgColor indexed="64"/></patternFill></fill></fills>'
        '<borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders>'
        '<cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs>'
        '<cellXfs count="4"><xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>'
        '<xf numFmtId="0" fontId="1" fillId="2" borderId="0" xfId="0" applyFont="1" applyFill="1"/>'
        '<xf numFmtId="164" fontId="0" fillId="0" borderId="0" xfId="0" applyNumberFormat="1"/>'
        '<xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0" applyAlignment="1">'
        '<alignment wrapText="1"/></xf></cellXfs>'
        '<cellStyles count="1"><cellStyle name="Normal" xfId="0" builtinId="0"/></cellStyles>'
        '<dxfs count="1"><dxf><fill><patternFill patternType="solid"><fgColor rgb="FFFFC7CE"/>'
        '<bgColor indexed="64"/></patternFill></fill></dxf></dxfs></styleSheet>'
    ).encode()


def drawing_xml() -> bytes:
    return (
        b'<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        b'<xdr:wsDr xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing" '
        b'xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" '
        b'xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" '
        b'xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">'
        b'<xdr:absoluteAnchor><xdr:pos x="0" y="0"/><xdr:ext cx="1800000" cy="900000"/>'
        b'<xdr:sp><xdr:nvSpPr><xdr:cNvPr id="2" name="Seed Shape"/><xdr:cNvSpPr/></xdr:nvSpPr>'
        b'<xdr:spPr><a:prstGeom prst="roundRect"><a:avLst/></a:prstGeom></xdr:spPr>'
        b"<xdr:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>Shape</a:t></a:r></a:p></xdr:txBody>"
        b"</xdr:sp><xdr:clientData/></xdr:absoluteAnchor>"
        b"<xdr:oneCellAnchor><xdr:from><xdr:col>0</xdr:col><xdr:colOff>0</xdr:colOff>"
        b'<xdr:row>7</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:from><xdr:ext cx="900000" cy="900000"/>'
        b'<xdr:pic><xdr:nvPicPr><xdr:cNvPr id="3" name="Seed Image"/><xdr:cNvPicPr/></xdr:nvPicPr>'
        b'<xdr:blipFill><a:blip r:embed="rId2"/><a:stretch><a:fillRect/></a:stretch></xdr:blipFill>'
        b'<xdr:spPr><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></xdr:spPr></xdr:pic>'
        b"<xdr:clientData/></xdr:oneCellAnchor>"
        b"<xdr:twoCellAnchor><xdr:from><xdr:col>3</xdr:col><xdr:colOff>0</xdr:colOff><xdr:row>0</xdr:row>"
        b"<xdr:rowOff>0</xdr:rowOff></xdr:from><xdr:to><xdr:col>10</xdr:col><xdr:colOff>0</xdr:colOff>"
        b'<xdr:row>15</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:to><xdr:graphicFrame macro="">'
        b'<xdr:nvGraphicFramePr><xdr:cNvPr id="4" name="Seed Chart"/><xdr:cNvGraphicFramePr/></xdr:nvGraphicFramePr>'
        b'<xdr:xfrm/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart">'
        b'<c:chart r:id="rId1"/></a:graphicData></a:graphic></xdr:graphicFrame><xdr:clientData/>'
        b"</xdr:twoCellAnchor></xdr:wsDr>"
    )


def chart_xml(case: XlsxCase) -> bytes:
    return (
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        '<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" '
        'xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><c:chart><c:plotArea>'
        '<c:layout/><c:barChart><c:barDir val="col"/><c:grouping val="clustered"/><c:ser>'
        '<c:idx val="0"/><c:order val="0"/><c:tx><c:v>Ordinal</c:v></c:tx><c:val><c:numLit>'
        f'<c:formatCode>General</c:formatCode><c:ptCount val="1"/><c:pt idx="0"><c:v>{case.ordinal}</c:v>'
        '</c:pt></c:numLit></c:val></c:ser><c:axId val="1"/><c:axId val="2"/></c:barChart>'
        '<c:catAx><c:axId val="1"/><c:scaling><c:orientation val="minMax"/></c:scaling>'
        '<c:axPos val="b"/><c:crossAx val="2"/></c:catAx>'
        '<c:valAx><c:axId val="2"/><c:scaling><c:orientation val="minMax"/></c:scaling>'
        '<c:axPos val="l"/><c:crossAx val="1"/></c:valAx>'
        "</c:plotArea></c:chart></c:chartSpace>"
    ).encode()


def seed_png(seed: str) -> bytes:
    rgb = bytes.fromhex(seed[:6])
    alternate = bytes(255 - channel for channel in rgb)
    rows = [
        b"\x00" + b"".join(rgb if (x + y) % 2 == 0 else alternate for x in range(8))
        for y in range(8)
    ]
    header = struct.pack(">IIBBBBB", 8, 8, 8, 2, 0, 0, 0)
    return (
        b"\x89PNG\r\n\x1a\n"
        + _png_chunk(b"IHDR", header)
        + _png_chunk(b"IDAT", zlib.compress(b"".join(rows), 9))
        + _png_chunk(b"IEND", b"")
    )


def _png_chunk(kind: bytes, value: bytes) -> bytes:
    checksum = binascii.crc32(kind + value) & 0xFFFFFFFF
    return struct.pack(">I", len(value)) + kind + value + struct.pack(">I", checksum)
