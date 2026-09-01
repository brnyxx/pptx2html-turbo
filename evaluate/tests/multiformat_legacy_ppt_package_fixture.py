from __future__ import annotations

import struct
import zlib
from typing import Final

_FREE: Final[int] = 0xFFFFFFFF
_END: Final[int] = 0xFFFFFFFE
_FAT: Final[int] = 0xFFFFFFFD


def make_package_zip(
    dos_date: int,
    dos_time: int,
    marker: int,
    *,
    chart: bool,
) -> bytes:
    mimetype = (
        b"application/vnd.oasis.opendocument.chart"
        if chart
        else b"application/vnd.oasis.opendocument.text"
    )
    members = (
        (b"mimetype", mimetype),
        (b"content.xml", f"<chart marker='{marker}'/>".encode()),
    )
    local = bytearray()
    central = bytearray()
    offsets: list[int] = []
    for ordinal, (name, data) in enumerate(members):
        offsets.append(len(local))
        crc = zlib.crc32(data)
        extra = struct.pack("<HHB", 0x5455, 1, marker + ordinal)
        local += (
            struct.pack(
                "<IHHHHHIIIHH",
                0x04034B50,
                20,
                0,
                0,
                dos_time,
                dos_date,
                crc,
                len(data),
                len(data),
                len(name),
                len(extra),
            )
            + name
            + extra
            + data
        )
    for ordinal, ((name, data), local_offset) in enumerate(
        zip(members, offsets, strict=True)
    ):
        crc = zlib.crc32(data)
        extra = struct.pack("<HHB", 0x5455, 1, marker + ordinal + 1)
        comment = f"member-{ordinal}".encode()
        central += (
            struct.pack(
                "<IHHHHHHIIIHHHHHII",
                0x02014B50,
                20,
                20,
                0,
                0,
                dos_time,
                dos_date,
                crc,
                len(data),
                len(data),
                len(name),
                len(extra),
                len(comment),
                0,
                0,
                0,
                local_offset,
            )
            + name
            + extra
            + comment
        )
    start = len(local)
    archive_comment = b"fixture-comment"
    return bytes(
        local
        + central
        + struct.pack(
            "<IHHHHIIH",
            0x06054B50,
            0,
            0,
            len(members),
            len(members),
            len(central),
            start,
            len(archive_comment),
        )
        + archive_comment
    )


def make_nested_cfb(package: bytes, stream_name: str) -> bytes:
    mini_count = (len(package) + 63) // 64
    mini_stream = package.ljust(512, b"\x00")
    directory = b"".join(
        (
            _entry("Root Entry", 5, child=1, start=1, size=512),
            _entry(stream_name, 2, start=0, size=len(package)),
        )
    ).ljust(512, b"\x00")
    mini_fat = bytearray(b"\xff" * 512)
    for index in range(mini_count):
        struct.pack_into(
            "<I",
            mini_fat,
            index * 4,
            _END if index + 1 == mini_count else index + 1,
        )
    return (
        _cfb_header()
        + directory
        + mini_stream
        + mini_fat
        + _fat_sector([_END, _END, _END, _FAT])
    )


def _cfb_header() -> bytes:
    header = bytearray(512)
    header[:8] = bytes.fromhex("d0cf11e0a1b11ae1")
    struct.pack_into("<HHHH", header, 24, 0x3E, 3, 0xFFFE, 9)
    struct.pack_into("<H", header, 32, 6)
    struct.pack_into("<III", header, 44, 1, 0, 0)
    struct.pack_into("<III", header, 56, 4096, 2, 1)
    struct.pack_into("<II", header, 68, _END, 0)
    for index in range(109):
        struct.pack_into("<I", header, 76 + index * 4, _FREE)
    struct.pack_into("<I", header, 76, 3)
    return bytes(header)


def _fat_sector(values: list[int]) -> bytes:
    result = bytearray(b"\xff" * 512)
    for index, value in enumerate(values):
        struct.pack_into("<I", result, index * 4, value)
    return bytes(result)


def _entry(
    name: str,
    kind: int,
    *,
    child: int = _FREE,
    start: int = _END,
    size: int = 0,
) -> bytes:
    result = bytearray(128)
    encoded = name.encode("utf-16le") + b"\x00\x00"
    result[: len(encoded)] = encoded
    struct.pack_into("<HBBIII", result, 64, len(encoded), kind, 1, _FREE, _FREE, child)
    struct.pack_into("<IQ", result, 116, start, size)
    return bytes(result)
