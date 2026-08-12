from __future__ import annotations

import base64
import binascii
import io
import struct
import zipfile
import zlib
from dataclasses import dataclass
from typing import Final, TypeAlias
from xml.sax.saxutils import quoteattr

if __package__:
    from .completion_deck_common import theme_xml
else:
    from completion_deck_common import theme_xml


Relationship: TypeAlias = tuple[str, str, str, str | None]
Part: TypeAlias = tuple[str, bytes]
ContentType: TypeAlias = tuple[str, str]
FIXED_TIME: Final = (1980, 1, 1, 0, 0, 0)
REL: Final = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/"
NS: Final = (
    'xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" '
    'xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" '
    'xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" '
    'xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" '
    'xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" '
    'xmlns:p188="http://schemas.microsoft.com/office/powerpoint/2018/8/main"'
)


@dataclass(frozen=True, slots=True)
class Deck:
    name: str
    slides: tuple[tuple[str, str], ...]
    slide_rels: tuple[Relationship, ...] = ()
    presentation_rels: tuple[Relationship, ...] = ()
    parts: tuple[Part, ...] = ()
    types: tuple[ContentType, ...] = ()
    backgrounds: tuple[str, ...] = ()
    slide_part_names: tuple[str, ...] = ()


def slide_part_names(deck: Deck) -> tuple[str, ...]:
    if deck.slide_part_names:
        if len(deck.slide_part_names) != len(deck.slides):
            raise ValueError("slide part names must match slide count")
        return deck.slide_part_names
    return tuple(f"slide{i}.xml" for i in range(1, len(deck.slides) + 1))


def relationships_xml(rows: tuple[Relationship, ...]) -> bytes:
    body = "".join(
        f"<Relationship Id={quoteattr(rid)} Type={quoteattr(kind)} "
        f"Target={quoteattr(target)}"
        f"{f' TargetMode={quoteattr(mode)}' if mode is not None else ''}/>"
        for rid, kind, target, mode in rows
    )
    return (
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
        f"{body}</Relationships>"
    ).encode()


def png_bytes() -> bytes:
    def chunk(kind: bytes, data: bytes) -> bytes:
        checksum = binascii.crc32(kind + data) & 0xFFFFFFFF
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", checksum)

    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", 1, 1, 8, 6, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(b"\x00\xff\x00\x00\xff", 9))
        + chunk(b"IEND", b"")
    )


def wav_bytes() -> bytes:
    samples = b"\0\0" * 80
    return (
        b"RIFF"
        + struct.pack("<I", 36 + len(samples))
        + b"WAVEfmt "
        + struct.pack("<IHHIIHH", 16, 1, 1, 8000, 16000, 2, 16)
        + b"data"
        + struct.pack("<I", len(samples))
        + samples
    )


def mp4_bytes() -> bytes:
    """Return a deterministic one-frame baseline-AVC MP4 fixture."""
    encoded = (
        "AAAAIGZ0eXBpc29tAAACAGlzb21pc28yYXZjMW1wNDEAAAMObW9vdgAAAGxtdmhkAAAAAAAAAAAAAAAAAAAD6AAAAMgAAQAAAQAAAAAAAAAAAAAAAAEAAAAAAAAAAAAAAAAAAAABAAAAAAAAAAAAAAAAAABAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAgAAAjl0cmFrAAAAXHRraGQAAAADAAAAAAAAAAAAAAABAAAAAAAAAMgAAAAAAAAAAAAAAAAAAAAAAAEAAAAAAAAAAAAAAAAAAAABAAAAAAAAAAAAAAAAAABAAAAAACAAAAAgAAAAAAAkZWR0cwAAABxlbHN0AAAAAAAAAAEAAADIAAAAAAABAAAAAAGxbWRpYQAAACBtZGhkAAAAAAAAAAAAAAAAAAAoAAAACABVxAAAAAAALWhkbHIAAAAAAAAAAHZpZGUAAAAAAAAAAAAAAABWaWRlb0hhbmRsZXIAAAABXG1pbmYAAAAUdm1oZAAAAAEAAAAAAAAAAAAAACRkaW5mAAAAHGRyZWYAAAAAAAAAAQAAAAx1cmwgAAAAAQAAARxzdGJsAAAAuHN0c2QAAAAAAAAAAQAAAKhhdmMxAAAAAAAAAAEAAAAAAAAAAAAAAAAAAAAAACAAIABIAAAASAAAAAAAAAABFUxhdmM2Mi4xMS4xMDAgbGlieDI2NAAAAAAAAAAAAAAAGP//AAAALmF2Y0MBQsAK/+EAFmdCwArZCWwEQAAAAwBAAAADAoPEiZIBAAVoy4PLIAAAABBwYXNwAAAAAQAAAAEAAAAUYnRydAAAAAAAAGYwAAAAAAAAABhzdHRzAAAAAAAAAAEAAAABAAAIAAAAABxzdHNjAAAAAAAAAAEAAAABAAAAAQAAAAEAAAAUc3RzegAAAAAAAAKOAAAAAQAAABRzdGNvAAAAAAAAAAEAAAM+AAAAYXVkdGEAAABZbWV0YQAAAAAAAAAhaGRscgAAAAAAAAAAbWRpcmFwcGwAAAAAAAAAAAAAAAAsaWxzdAAAACSpdG9vAAAAHGRhdGEAAAABAAAAAExhdmY2Mi4zLjEwMAAAAAhmcmVlAAAClm1kYXQAAAJwBgX//2zcRem95tlIt5Ys2CDZI+7veDI2NCAtIGNvcmUgMTY1IHIzMjIyIGIzNTYwNWEgLSBILjI2NC9NUEVHLTQgQVZDIGNvZGVjIC0gQ29weWxlZnQgMjAwMy0yMDI1IC0gaHR0cDovL3d3dy52aWRlb2xhbi5vcmcveDI2NC5odG1sIC0gb3B0aW9uczogY2FiYWM9MCByZWY9MyBkZWJsb2NrPTE6MDowIGFuYWx5c2U9MHgxOjB4MTExIG1lPWhleCBzdWJtZT03IHBzeT0xIHBzeV9yZD0xLjAwOjAuMDAgbWl4ZWRfcmVmPTEgbWVfcmFuZ2U9MTYgY2hyb21hX21lPTEgdHJlbGxpcz0xIDh4OGRjdD0wIGNxbT0wIGRlYWR6b25lPTIxLDExIGZhc3RfcHNraXA9MSBjaHJvbWFfcXBfb2Zmc2V0PS0yIHRocmVhZHM9MSBsb29rYWhlYWRfdGhyZWFkcz0xIHNsaWNlZF90aHJlYWRzPTAgbnI9MCBkZWNpbWF0ZT0xIGludGVybGFjZWQ9MCBibHVyYXlfY29tcGF0PTAgY29uc3RyYWluZWRfaW50cmE9MCBiZnJhbWVzPTAgd2VpZ2h0cD0wIGtleWludD0yNTAga2V5aW50X21pbj01IHNjZW5lY3V0PTQwIGludHJhX3JlZnJlc2g9MCByY19sb29rYWhlYWQ9NDAgcmM9Y3JmIG1idHJlZT0xIGNyZj0yMy4wIHFjb21wPTAuNjAgcXBtaW49MCBxcG1heD02OSBxcHN0ZXA9NCBpcF9yYXRpbz0xLjQwIGFxPTE6MS4wMACAAAAAFmWIhA/xGKAALSMcAAVco4AAhgyddeA="
    )
    return base64.b64decode(encoded, validate=True)


def deck_bytes(deck: Deck) -> bytes:
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, "w", zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
        for name, data in sorted(_package_parts(deck).items()):
            info = zipfile.ZipInfo(name, FIXED_TIME)
            info.compress_type = zipfile.ZIP_DEFLATED
            info.create_system = 0
            info.external_attr = 0
            archive.writestr(info, data)
    return buffer.getvalue()


def _slide(body: str, tail: str, background: str = "") -> bytes:
    return (
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        f"<p:sld {NS}><p:cSld>{background}<p:spTree><p:nvGrpSpPr>"
        '<p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>'
        '<p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/>'
        '<a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr>'
        f"{body}</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr>"
        f"{tail}</p:sld>"
    ).encode()


def _content_types(deck: Deck) -> bytes:
    common = (
        (
            "/ppt/presentation.xml",
            "application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml",
        ),
        (
            "/ppt/presProps.xml",
            "application/vnd.openxmlformats-officedocument.presentationml.presProps+xml",
        ),
        (
            "/ppt/slideMasters/slideMaster1.xml",
            "application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml",
        ),
        (
            "/ppt/slideLayouts/slideLayout1.xml",
            "application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml",
        ),
        (
            "/ppt/theme/theme1.xml",
            "application/vnd.openxmlformats-officedocument.theme+xml",
        ),
    )
    slides = tuple(
        (
            f"/ppt/slides/{name}",
            "application/vnd.openxmlformats-officedocument.presentationml.slide+xml",
        )
        for name in slide_part_names(deck)
    )
    overrides = "".join(
        f"<Override PartName={quoteattr(name)} ContentType={quoteattr(kind)}/>"
        for name, kind in (*common, *deck.types, *slides)
    )
    return (
        '<?xml version="1.0"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
        '<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>'
        '<Default Extension="xml" ContentType="application/xml"/>'
        '<Default Extension="png" ContentType="image/png"/>'
        '<Default Extension="wav" ContentType="audio/wav"/>'
        '<Default Extension="mp4" ContentType="video/mp4"/>'
        '<Default Extension="bin" ContentType="application/octet-stream"/>'
        f"{overrides}</Types>"
    ).encode()


def _package_parts(deck: Deck) -> dict[str, bytes]:
    names = slide_part_names(deck)
    slide_ids = "".join(
        f"<p:sldId id={quoteattr(str(255 + i))} r:id={quoteattr(f'rIdSlide{i}')}/>"
        for i in range(1, len(deck.slides) + 1)
    )
    presentation_rels: tuple[Relationship, ...] = (
        ("rIdMaster", REL + "slideMaster", "slideMasters/slideMaster1.xml", None),
        ("rIdPresProps", REL + "presProps", "presProps.xml", None),
        *(
            (f"rIdSlide{i}", REL + "slide", f"slides/{name}", None)
            for i, name in enumerate(names, 1)
        ),
        *deck.presentation_rels,
    )
    parts = {
        "[Content_Types].xml": _content_types(deck),
        "_rels/.rels": relationships_xml(
            (("rId1", REL + "officeDocument", "ppt/presentation.xml", None),)
        ),
        "ppt/presentation.xml": (
            f'<?xml version="1.0"?><p:presentation {NS}><p:sldMasterIdLst><p:sldMasterId id="2147483648" r:id="rIdMaster"/></p:sldMasterIdLst><p:sldIdLst>{slide_ids}</p:sldIdLst><p:sldSz cx="9144000" cy="6858000"/><p:notesSz cx="6858000" cy="9144000"/><p:defaultTextStyle/></p:presentation>'
        ).encode(),
        "ppt/_rels/presentation.xml.rels": relationships_xml(presentation_rels),
        "ppt/presProps.xml": f'<?xml version="1.0"?><p:presentationPr {NS}/>'.encode(),
        "ppt/slideMasters/slideMaster1.xml": (
            f'<?xml version="1.0"?><p:sldMaster {NS}><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld><p:clrMap accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" bg1="lt1" bg2="lt2" folHlink="folHlink" hlink="hlink" tx1="dk1" tx2="dk2"/><p:sldLayoutIdLst><p:sldLayoutId id="2147483648" r:id="rIdLayout"/></p:sldLayoutIdLst><p:txStyles><p:titleStyle/><p:bodyStyle/><p:otherStyle/></p:txStyles></p:sldMaster>'
        ).encode(),
        "ppt/slideMasters/_rels/slideMaster1.xml.rels": relationships_xml(
            (
                (
                    "rIdLayout",
                    REL + "slideLayout",
                    "../slideLayouts/slideLayout1.xml",
                    None,
                ),
                ("rIdTheme", REL + "theme", "../theme/theme1.xml", None),
            )
        ),
        "ppt/slideLayouts/slideLayout1.xml": (
            f'<?xml version="1.0"?><p:sldLayout {NS} type="blank"><p:cSld name="Blank"><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sldLayout>'
        ).encode(),
        "ppt/slideLayouts/_rels/slideLayout1.xml.rels": relationships_xml(
            (
                (
                    "rIdMaster",
                    REL + "slideMaster",
                    "../slideMasters/slideMaster1.xml",
                    None,
                ),
            )
        ),
        "ppt/theme/theme1.xml": theme_xml(),
        **dict(deck.parts),
    }
    for index, ((body, tail), name) in enumerate(
        zip(deck.slides, names, strict=True), 1
    ):
        background = (
            deck.backgrounds[index - 1] if index <= len(deck.backgrounds) else ""
        )
        parts[f"ppt/slides/{name}"] = _slide(body, tail, background)
        feature_rels = deck.slide_rels if index == 1 else ()
        parts[f"ppt/slides/_rels/{name}.rels"] = relationships_xml(
            (
                (
                    "rIdLayout",
                    REL + "slideLayout",
                    "../slideLayouts/slideLayout1.xml",
                    None,
                ),
                *feature_rels,
            )
        )
    return parts
