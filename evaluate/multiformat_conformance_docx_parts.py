from __future__ import annotations

import binascii
import struct
import zlib
from dataclasses import dataclass
from enum import StrEnum
from html import escape
from typing import Final, assert_never

from evaluate.multiformat_schema import JsonValue, integer_value, string_value

CONTENT_TYPES: Final[
    bytes
] = b"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Default Extension="png" ContentType="image/png"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/><Override PartName="/word/numbering.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml"/><Override PartName="/word/header1.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml"/><Override PartName="/word/footer1.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.footer+xml"/></Types>"""

ROOT_RELS: Final[bytes] = b"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"""

STYLES: Final[bytes] = b"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii="Liberation Sans" w:hAnsi="Liberation Sans" w:eastAsia="Noto Sans CJK KR" w:cs="Noto Naskh Arabic"/><w:sz w:val="22"/></w:rPr></w:rPrDefault></w:docDefaults><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/><w:qFormat/></w:style><w:style w:type="paragraph" w:styleId="SnapshotTitle"><w:name w:val="Snapshot Title"/><w:basedOn w:val="Normal"/><w:qFormat/><w:rPr><w:b/><w:color w:val="17365D"/><w:sz w:val="32"/></w:rPr></w:style></w:styles>"""

NUMBERING: Final[bytes] = b"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:abstractNum w:abstractNumId="0"><w:multiLevelType w:val="singleLevel"/><w:lvl w:ilvl="0"><w:start w:val="1"/><w:numFmt w:val="decimal"/><w:lvlText w:val="%1."/><w:pPr><w:ind w:left="720" w:hanging="360"/></w:pPr></w:lvl></w:abstractNum><w:num w:numId="1"><w:abstractNumId w:val="0"/></w:num></w:numbering>"""

DOCUMENT_RELS: Final[
    bytes
] = b"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/numbering" Target="numbering.xml"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="header1.xml"/><Relationship Id="rId4" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/footer" Target="footer1.xml"/><Relationship Id="rId5" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/image1.png"/></Relationships>"""


class DocxPartError(Exception):
    pass


@dataclass(frozen=True, slots=True)
class DocxCase:
    case_id: str
    ordinal: int
    stratum: DocxStratum
    seed: str


class DocxStratum(StrEnum):
    TEXT_TYPOGRAPHY = "text-typography"
    SECTIONS_HEADERS_FOOTERS = "sections-headers-footers"
    TABLES_IMAGES_SHAPES = "tables-images-shapes"
    LISTS_FIELDS_REFERENCES = "lists-fields-references"
    INTERNATIONAL = "international"
    MIXED_STRESS = "mixed-stress"


def case_parts(case: dict[str, JsonValue]) -> tuple[tuple[str, bytes], ...]:
    case_id = string_value(case, "id")
    ordinal = integer_value(case, "ordinal")
    stratum = DocxStratum(string_value(case, "primary_stratum"))
    seed = string_value(case, "feature_seed")
    if len(seed) != 64:
        raise DocxPartError("DOCX feature seed must be SHA-256")
    return (
        ("[Content_Types].xml", CONTENT_TYPES),
        ("_rels/.rels", ROOT_RELS),
        ("word/document.xml", _document(DocxCase(case_id, ordinal, stratum, seed))),
        ("word/styles.xml", STYLES),
        ("word/numbering.xml", NUMBERING),
        ("word/header1.xml", _header(case_id, stratum)),
        ("word/footer1.xml", _footer(case_id, ordinal)),
        ("word/_rels/document.xml.rels", DOCUMENT_RELS),
        ("word/media/image1.png", _png(seed)),
    )


def _document(case: DocxCase) -> bytes:
    case_id = case.case_id
    ordinal = case.ordinal
    stratum = case.stratum
    seed = case.seed
    feature = escape(_feature_text(stratum, ordinal))
    safe_id = escape(case_id)
    safe_stratum = escape(stratum)
    color = seed[:6].upper()
    page_width, page_height, orientation = (
        ("15840", "12240", ' w:orient="landscape"')
        if ordinal % 2 == 0
        else ("12240", "15840", "")
    )
    value = f'''<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture" xmlns:v="urn:schemas-microsoft-com:vml" xmlns:o="urn:schemas-microsoft-com:office:office"><w:body><w:p><w:pPr><w:pStyle w:val="SnapshotTitle"/></w:pPr><w:r><w:t>{safe_id}</w:t></w:r></w:p><w:p><w:r><w:rPr><w:b/><w:i/><w:color w:val="{color}"/><w:sz w:val="28"/></w:rPr><w:t>{safe_stratum} | ordinal {ordinal} | seed {seed}</w:t></w:r></w:p><w:p><w:r><w:t>{feature}</w:t></w:r></w:p><w:p><w:r><w:t xml:space="preserve">한국어 日本語 العربية Español – naïve façade</w:t></w:r></w:p><w:p><w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="1"/></w:numPr></w:pPr><w:r><w:t>Planned list item {ordinal}</w:t></w:r></w:p><w:p><w:fldSimple w:instr=" DATE &#92;@ &quot;yyyy-MM-dd&quot; " w:fldLock="true"><w:r><w:t>2000-01-01</w:t></w:r></w:fldSimple></w:p><w:tbl><w:tblPr><w:tblStyle w:val="TableGrid"/></w:tblPr><w:tblGrid><w:gridCol w:w="3600"/><w:gridCol w:w="3600"/></w:tblGrid><w:tr><w:tc><w:p><w:r><w:t>Case</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>{safe_id}</w:t></w:r></w:p></w:tc></w:tr></w:tbl><w:p><w:r><w:drawing><wp:inline><wp:extent cx="914400" cy="457200"/><wp:docPr id="1" name="Seed image"/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture"><pic:pic><pic:nvPicPr><pic:cNvPr id="2" name="image1.png"/><pic:cNvPicPr/></pic:nvPicPr><pic:blipFill><a:blip r:embed="rId5"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill><pic:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="457200"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></pic:spPr></pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing></w:r></w:p><w:p><w:r><w:pict><v:shape id="SnapshotShape" type="#_x0000_t202" style="width:180pt;height:36pt" fillcolor="#{color}"><v:textbox><w:txbxContent><w:p><w:r><w:t>Deterministic shape {ordinal}</w:t></w:r></w:p></w:txbxContent></v:textbox></v:shape></w:pict></w:r></w:p><w:sectPr><w:headerReference w:type="default" r:id="rId3"/><w:footerReference w:type="default" r:id="rId4"/><w:pgSz w:w="{page_width}" w:h="{page_height}"{orientation}/><w:pgMar w:top="720" w:right="720" w:bottom="720" w:left="720" w:header="360" w:footer="360" w:gutter="0"/></w:sectPr></w:body></w:document>'''
    return value.encode()


def _feature_text(stratum: DocxStratum, ordinal: int) -> str:
    match stratum:
        case DocxStratum.TEXT_TYPOGRAPHY:
            return "Bold italic colored typography with inherited fonts"
        case DocxStratum.SECTIONS_HEADERS_FOOTERS:
            return f"Section geometry and linked running content {ordinal}"
        case DocxStratum.TABLES_IMAGES_SHAPES:
            return "Bordered table, seeded raster image, and VML text shape"
        case DocxStratum.LISTS_FIELDS_REFERENCES:
            return "Numbered paragraph and deterministic DATE field result"
        case DocxStratum.INTERNATIONAL:
            return "Multiscript runs: 한국어 日本語 العربية Español"
        case DocxStratum.MIXED_STRESS:
            return "Mixed typography, structures, drawing, field, and scripts"
        case unreachable:
            assert_never(unreachable)


def _header(case_id: str, stratum: DocxStratum) -> bytes:
    return f"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:hdr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:p><w:r><w:t>{escape(case_id)} header | {escape(stratum)}</w:t></w:r></w:p></w:hdr>""".encode()


def _footer(case_id: str, ordinal: int) -> bytes:
    return f"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:ftr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:p><w:r><w:t>{escape(case_id)} footer {ordinal}</w:t></w:r></w:p></w:ftr>""".encode()


def _png(seed: str) -> bytes:
    color = bytes.fromhex(seed[:6])
    rows = b"".join(b"\x00" + color * 8 for _ in range(4))
    header = struct.pack(">IIBBBBB", 8, 4, 8, 2, 0, 0, 0)
    return (
        b"\x89PNG\r\n\x1a\n"
        + _chunk(b"IHDR", header)
        + _chunk(b"IDAT", zlib.compress(rows, 9))
        + _chunk(b"IEND", b"")
    )


def _chunk(kind: bytes, value: bytes) -> bytes:
    return (
        struct.pack(">I", len(value))
        + kind
        + value
        + struct.pack(">I", binascii.crc32(kind + value) & 0xFFFFFFFF)
    )
