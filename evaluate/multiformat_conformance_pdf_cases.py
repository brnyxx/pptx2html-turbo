from __future__ import annotations

import base64
import binascii
import html
import struct
import zlib

from evaluate.multiformat_conformance_pdf import PdfConformanceError
from evaluate.multiformat_schema import JsonValue, integer_value, string_value


def pdf_case_html(case: dict[str, JsonValue]) -> bytes:
    case_id = string_value(case, "id")
    ordinal = integer_value(case, "ordinal")
    stratum = string_value(case, "primary_stratum")
    seed = string_value(case, "feature_seed")
    color = f"#{seed[:6]}"
    feature = _feature_html(stratum, case_id, ordinal, color, seed)
    page_rule = (
        "@page { size: A4 landscape; margin: 18mm; }"
        if stratum == "page-geometry"
        else "@page { size: A4 portrait; margin: 18mm; }"
    )
    value = (
        '<!doctype html><html><head><meta charset="utf-8"><style>'
        f"{page_rule}"
        "body{font-family:'Noto Sans','Apple SD Gothic Neo','Hiragino Sans GB',"
        "'Amiri',sans-serif;color:#172033;"
        "font-size:14pt;line-height:1.35}"
        "h1{font-size:24pt;margin:0 0 12pt}"
        ".card{border:2pt solid #324d70;padding:12pt;background:#f5f8fc}"
        "table{border-collapse:collapse}td,th{border:1pt solid #445;padding:6pt}"
        "</style></head><body>"
        f"<h1>{html.escape(case_id)}</h1>"
        f"<p>stratum: {html.escape(stratum)} / ordinal: {ordinal}</p>"
        f"{feature}</body></html>"
    )
    return value.encode("utf-8")


def _feature_html(
    stratum: str,
    case_id: str,
    ordinal: int,
    color: str,
    seed: str,
) -> str:
    escaped_id = html.escape(case_id)
    if stratum == "text-fonts":
        return (
            '<div class="card font-variant">'
            f"<p><b>Bold {escaped_id}</b> <i>Italic</i> "
            '<span style="font-variant:small-caps;letter-spacing:2pt">'
            "Small Caps</span></p></div>"
        )
    if stratum == "vector-transparency":
        return (
            '<svg width="480" height="180" viewBox="0 0 480 180">'
            f'<rect x="20" y="20" width="280" height="120" fill="{color}"/>'
            '<circle cx="300" cy="90" r="70" fill="#e34b4b" opacity="0.55"/>'
            "</svg>"
        )
    if stratum == "raster-color-space":
        png = base64.b64encode(_tiny_png(seed)).decode()
        return (
            '<div class="card"><p>RGB raster and grayscale contrast</p>'
            f'<img alt="{escaped_id}" width="240" height="120" '
            f'src="data:image/png;base64,{png}"></div>'
        )
    if stratum == "page-geometry":
        return (
            '<div class="card landscape" style="width:85%;height:260px">'
            f"Landscape geometry {escaped_id}</div>"
        )
    if stratum == "forms-annotations-links":
        return (
            '<div class="card"><label><input type="checkbox" checked> Accepted</label>'
            '<p><a href="https://example.com/conformance">'
            f"Linked annotation {escaped_id}</a></p></div>"
        )
    if stratum == "international":
        return (
            '<div class="card" lang="ko"><p style="font-family:\'Apple SD '
            'Gothic Neo\'">한글 문서 정확도</p><p lang="ja" '
            "style=\"font-family:'Hiragino Sans GB'\">日本語の文書</p>"
            '<p dir="rtl" lang="ar" style="font-family:\'Amiri\'">'
            f"العربية {ordinal}</p><p>Español – naïve façade</p></div>"
        )
    if stratum == "mixed-edge":
        return (
            '<table class="mixed-edge-table"><tr><th>Case</th><th>Value</th></tr>'
            f"<tr><td>{escaped_id}</td><td>{ordinal:,}</td></tr>"
            '<tr><td dir="rtl">مختلط</td><td><b>Bold</b> + link</td></tr></table>'
        )
    raise PdfConformanceError(f"unsupported PDF stratum: {stratum}")


def _tiny_png(seed: str) -> bytes:
    width = 8
    height = 4
    rgb = bytes.fromhex(seed[:6])
    alternate = bytes(255 - value for value in rgb)
    rows = []
    for y in range(height):
        pixels = b"".join(rgb if (x + y) % 2 == 0 else alternate for x in range(width))
        rows.append(b"\x00" + pixels)
    header = struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0)
    return (
        b"\x89PNG\r\n\x1a\n"
        + _png_chunk(b"IHDR", header)
        + _png_chunk(b"IDAT", zlib.compress(b"".join(rows), level=9))
        + _png_chunk(b"IEND", b"")
    )


def _png_chunk(chunk_type: bytes, data: bytes) -> bytes:
    return (
        struct.pack(">I", len(data))
        + chunk_type
        + data
        + struct.pack(">I", binascii.crc32(chunk_type + data) & 0xFFFFFFFF)
    )
