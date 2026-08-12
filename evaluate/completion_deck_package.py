from __future__ import annotations

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


class _BitWriter:
    def __init__(self) -> None:
        self.data = bytearray()
        self.position = 0

    def bit(self, value: int) -> None:
        if self.position % 8 == 0:
            self.data.append(0)
        if value:
            self.data[-1] |= 1 << (7 - self.position % 8)
        self.position += 1

    def bits(self, value: int, count: int) -> None:
        for shift in reversed(range(count)):
            self.bit((value >> shift) & 1)

    def ue(self, value: int) -> None:
        code = value + 1
        width = code.bit_length()
        for _ in range(width - 1):
            self.bit(0)
        self.bits(code, width)

    def se(self, value: int) -> None:
        self.ue(-2 * value if value <= 0 else 2 * value - 1)

    def align_zero(self) -> None:
        while self.position % 8:
            self.bit(0)

    def raw(self, values: bytes) -> None:
        assert self.position % 8 == 0
        self.data.extend(values)
        self.position += len(values) * 8

    def finish_rbsp(self) -> bytes:
        self.bit(1)
        self.align_zero()
        return bytes(self.data)


def _ebsp_nal(header: int, rbsp: bytes) -> bytes:
    result = bytearray([header])
    zeros = 0
    for value in rbsp:
        if zeros >= 2 and value <= 3:
            result.append(3)
            zeros = 0
        result.append(value)
        zeros = zeros + 1 if value == 0 else 0
    return bytes(result)


def _mp4_box(kind: bytes, payload: bytes) -> bytes:
    return struct.pack(">I4s", len(payload) + 8, kind) + payload


def mp4_bytes(width_mbs: int = 2, height_mbs: int = 1, pixel_seed: int = 0x31) -> bytes:
    """Build a deterministic single-frame Constrained-Baseline all-I_PCM MP4."""
    assert 1 <= width_mbs <= 16 and 1 <= height_mbs <= 16
    width, height = width_mbs * 16, height_mbs * 16

    sps_bits = _BitWriter()
    sps_bits.bits(66, 8)
    sps_bits.bits(0xC0, 8)
    sps_bits.bits(30, 8)
    for value in (0, 0, 0, 0, 1):
        sps_bits.ue(value)
    sps_bits.bit(0)
    sps_bits.ue(width_mbs - 1)
    sps_bits.ue(height_mbs - 1)
    for value in (1, 1, 0, 0):
        sps_bits.bit(value)
    sps = _ebsp_nal(0x67, sps_bits.finish_rbsp())

    pps_bits = _BitWriter()
    pps_bits.ue(0)
    pps_bits.ue(0)
    pps_bits.bit(0)
    pps_bits.bit(0)
    for _ in range(3):
        pps_bits.ue(0)
    pps_bits.bit(0)
    pps_bits.bits(0, 2)
    for _ in range(3):
        pps_bits.se(0)
    for value in (0, 0, 0):
        pps_bits.bit(value)
    pps = _ebsp_nal(0x68, pps_bits.finish_rbsp())

    slice_bits = _BitWriter()
    for value in (0, 7, 0):
        slice_bits.ue(value)
    slice_bits.bits(0, 4)
    slice_bits.ue(0)
    slice_bits.bits(0, 4)
    slice_bits.bit(0)
    slice_bits.bit(0)
    slice_bits.se(0)
    for macroblock in range(width_mbs * height_mbs):
        slice_bits.ue(25)
        slice_bits.align_zero()
        slice_bits.raw(
            bytes(
                (pixel_seed + macroblock * 17 + index) & 0xFF
                for index in range(384)
            )
        )
    idr = _ebsp_nal(0x65, slice_bits.finish_rbsp())
    sample = struct.pack(">I", len(idr)) + idr

    avcc = (
        bytes([1, 66, 0xC0, 30, 0xFF, 0xE1])
        + struct.pack(">H", len(sps))
        + sps
        + bytes([1])
        + struct.pack(">H", len(pps))
        + pps
    )
    compressor = bytes([0]) + bytes(31)
    avc1 = _mp4_box(
        b"avc1",
        bytes(6)
        + struct.pack(">H", 1)
        + bytes(16)
        + struct.pack(">HHII", width, height, 72 << 16, 72 << 16)
        + bytes(4)
        + struct.pack(">H", 1)
        + compressor
        + struct.pack(">Hh", 24, -1)
        + _mp4_box(b"avcC", avcc),
    )
    stsd = _mp4_box(b"stsd", bytes(4) + struct.pack(">I", 1) + avc1)
    stts = _mp4_box(b"stts", bytes(4) + struct.pack(">III", 1, 1, 1000))
    stsc = _mp4_box(b"stsc", bytes(4) + struct.pack(">IIII", 1, 1, 1, 1))
    stsz = _mp4_box(b"stsz", bytes(4) + struct.pack(">III", 0, 1, len(sample)))
    stss = _mp4_box(b"stss", bytes(4) + struct.pack(">II", 1, 1))
    ftyp = _mp4_box(b"ftyp", b"isom" + struct.pack(">I", 0x200) + b"isomavc1mp41")

    def moov(chunk_offset: int) -> bytes:
        stco = _mp4_box(b"stco", bytes(4) + struct.pack(">II", 1, chunk_offset))
        stbl = _mp4_box(b"stbl", stsd + stts + stsc + stsz + stco + stss)
        url = _mp4_box(b"url ", b"\0\0\0\1")
        dref = _mp4_box(b"dref", bytes(4) + struct.pack(">I", 1) + url)
        dinf = _mp4_box(b"dinf", dref)
        vmhd = _mp4_box(b"vmhd", b"\0\0\0\1" + bytes(8))
        minf = _mp4_box(b"minf", vmhd + dinf + stbl)
        mdhd = _mp4_box(b"mdhd", bytes(12) + struct.pack(">IIHH", 1000, 1000, 0x55C4, 0))
        hdlr = _mp4_box(b"hdlr", bytes(8) + b"vide" + bytes(12) + b"Video\0")
        mdia = _mp4_box(b"mdia", mdhd + hdlr + minf)
        matrix = struct.pack(">9I", 0x10000, 0, 0, 0, 0x10000, 0, 0, 0, 0x40000000)
        tkhd = _mp4_box(
            b"tkhd",
            b"\0\0\0\3"
            + bytes(8)
            + struct.pack(">II", 1, 0)
            + struct.pack(">I", 1000)
            + bytes(8)
            + struct.pack(">hhhh", 0, 0, 0, 0)
            + matrix
            + struct.pack(">II", width << 16, height << 16),
        )
        trak = _mp4_box(b"trak", tkhd + mdia)
        mvhd = _mp4_box(
            b"mvhd",
            bytes(12)
            + struct.pack(">II", 1000, 1000)
            + struct.pack(">Ih", 0x10000, 0x100)
            + bytes(10)
            + matrix
            + bytes(24)
            + struct.pack(">I", 2),
        )
        return _mp4_box(b"moov", mvhd + trak)

    placeholder = moov(0)
    movie = moov(len(ftyp) + len(placeholder) + 8)
    return ftyp + movie + _mp4_box(b"mdat", sample)


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
