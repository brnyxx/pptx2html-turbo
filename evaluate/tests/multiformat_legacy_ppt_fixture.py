from __future__ import annotations

import struct
import zlib
from dataclasses import dataclass
from typing import Final

from evaluate.tests.multiformat_legacy_ppt_package_fixture import (
    make_nested_cfb,
    make_package_zip,
)

FREE: Final[int] = 0xFFFFFFFF
END: Final[int] = 0xFFFFFFFE
FAT: Final[int] = 0xFFFFFFFD
EX_OLE: Final[int] = 0x1011
PERSIST: Final[int] = 0x1772
USER_EDIT: Final[int] = 0x0FF5
CURRENT_USER: Final[int] = 0x0FF6


@dataclass(frozen=True, slots=True)
class LegacyPptFixture:
    value: bytes
    package_zips: tuple[bytes, ...]


def make_legacy_ppt_fixture(
    dos_date: int,
    dos_time: int,
    *,
    include_packages: bool = True,
    compressed: bool = True,
    chart_package: bool = True,
    malformed_package: bool = False,
) -> LegacyPptFixture:
    packages = tuple(
        make_package_zip(
            dos_date,
            dos_time,
            index,
            chart=chart_package,
        )
        for index in range(3)
    )
    if malformed_package:
        packages = tuple(package[:-1] for package in packages)
    stream_name = "package_stream" if include_packages else "object_stream"
    nested = tuple(make_nested_cfb(package, stream_name) for package in packages)
    records = bytearray()
    edits: list[int] = []
    previous_edit = 0
    for index, storage in enumerate(nested, 1):
        ex_offset = len(records)
        body = (
            struct.pack("<I", len(storage)) + zlib.compress(storage, level=9)
            if compressed
            else storage
        )
        records += _record(EX_OLE, body, options=0x0010 if compressed else 0)
        persist_offset = len(records)
        records += _record(PERSIST, struct.pack("<II", (1 << 20) | index, ex_offset))
        edit_offset = len(records)
        body = bytearray(28)
        struct.pack_into("<II", body, 8, previous_edit, persist_offset)
        struct.pack_into("<II", body, 16, index, index + 1)
        records += _record(USER_EDIT, bytes(body))
        edits.append(edit_offset)
        previous_edit = edit_offset
    records += _record(0x2223, b"", options=0x000F)
    records += _record(0x2222, b"N" * 3_200)
    current_user = bytearray(_record(CURRENT_USER, b"\x00" * 24))
    struct.pack_into("<I", current_user, 16, edits[-1])
    return LegacyPptFixture(
        _outer_cfb(bytes(records), bytes(current_user)),
        packages,
    )


def extract_package_zips(value: bytes) -> tuple[bytes, ...]:
    power_point = _regular_stream(value, "PowerPoint Document")
    result: list[bytes] = []
    offset = 0
    while offset < len(power_point):
        _, record_type, length = struct.unpack_from("<HHI", power_point, offset)
        body = power_point[offset + 8 : offset + 8 + length]
        if record_type == EX_OLE:
            storage = zlib.decompress(body[4:])
            result.append(_mini_stream(storage, "package_stream"))
        offset += 8 + length
    return tuple(result)


def current_user_target_type(value: bytes) -> int:
    stream = _regular_stream(value, "PowerPoint Document")
    current = _mini_stream(value, "Current User")
    target = struct.unpack_from("<I", current, 16)[0]
    return struct.unpack_from("<H", stream, target + 2)[0]


def ppt_offsets(value: bytes) -> tuple[tuple[int, ...], tuple[int, ...], int]:
    stream = _regular_stream(value, "PowerPoint Document")
    persists: list[int] = []
    edits: list[int] = []
    offset = 0
    while offset < len(stream):
        _, kind, length = struct.unpack_from("<HHI", stream, offset)
        body = stream[offset + 8 : offset + 8 + length]
        if kind == PERSIST:
            persists.append(struct.unpack_from("<I", body, 4)[0])
        elif kind == USER_EDIT:
            edits.extend(struct.unpack_from("<II", body, 8))
        offset += 8 + length
    current = _mini_stream(value, "Current User")
    return tuple(persists), tuple(edits), struct.unpack_from("<I", current, 16)[0]


def _record(kind: int, body: bytes, *, options: int = 0) -> bytes:
    return struct.pack("<HHI", options, kind, len(body)) + body


def _outer_cfb(power_point: bytes, current_user: bytes) -> bytes:
    capacity_sectors = max(9, (len(power_point) + 511) // 512)
    root_sector = capacity_sectors + 1
    mini_fat_sector = root_sector + 1
    fat_id = mini_fat_sector + 1
    mini_count = (len(current_user) + 63) // 64
    entries = [
        _entry("Root Entry", 5, child=1, start=root_sector, size=512),
        _entry("PowerPoint Document", 2, right=2, start=1, size=len(power_point)),
        _entry("Current User", 2, start=0, size=len(current_user)),
    ]
    directory = b"".join(entries).ljust(512, b"\x00")
    regular = power_point.ljust(capacity_sectors * 512, b"\x00")
    mini_stream = current_user.ljust(512, b"\x00")
    mini_fat = bytearray(b"\xff" * 512)
    for index in range(mini_count):
        struct.pack_into(
            "<I", mini_fat, index * 4, END if index + 1 == mini_count else index + 1
        )
    fat_values = [END] + list(range(2, capacity_sectors + 1)) + [END, END, END, FAT]
    return (
        _cfb_header(fat_id, 0, mini_fat_sector, 1)
        + directory
        + regular
        + mini_stream
        + mini_fat
        + _fat_sector(fat_values)
    )


def _cfb_header(fat_id: int, directory: int, mini_fat: int, mini_count: int) -> bytes:
    header = bytearray(512)
    header[:8] = bytes.fromhex("d0cf11e0a1b11ae1")
    struct.pack_into("<HHHH", header, 24, 0x3E, 3, 0xFFFE, 9)
    struct.pack_into("<H", header, 32, 6)
    struct.pack_into("<III", header, 44, 1, directory, 0)
    struct.pack_into("<III", header, 56, 4096, mini_fat, mini_count)
    struct.pack_into("<II", header, 68, END, 0)
    for index in range(109):
        struct.pack_into("<I", header, 76 + index * 4, FREE)
    struct.pack_into("<I", header, 76, fat_id)
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
    child: int = FREE,
    right: int = FREE,
    start: int = END,
    size: int = 0,
) -> bytes:
    result = bytearray(128)
    encoded = name.encode("utf-16le") + b"\x00\x00"
    result[: len(encoded)] = encoded
    struct.pack_into("<HBBIII", result, 64, len(encoded), kind, 1, FREE, right, child)
    struct.pack_into("<IQ", result, 116, start, size)
    return bytes(result)


def _directory(value: bytes) -> bytes:
    sector = struct.unpack_from("<I", value, 48)[0]
    return value[(sector + 1) * 512 : (sector + 2) * 512]


def _find_entry(value: bytes, name: str) -> tuple[int, int]:
    directory = _directory(value)
    for offset in range(0, len(directory), 128):
        entry = directory[offset : offset + 128]
        length = struct.unpack_from("<H", entry, 64)[0]
        if length >= 2 and entry[: length - 2].decode("utf-16le") == name:
            return struct.unpack_from("<IQ", entry, 116)
    raise AssertionError(f"missing stream {name}")


def _regular_stream(value: bytes, name: str) -> bytes:
    start, size = _find_entry(value, name)
    fat = value[-512:]
    chunks: list[bytes] = []
    while start != END:
        chunks.append(value[(start + 1) * 512 : (start + 2) * 512])
        start = struct.unpack_from("<I", fat, start * 4)[0]
    return b"".join(chunks)[:size]


def _mini_stream(value: bytes, name: str) -> bytes:
    start, size = _find_entry(value, name)
    root_start, root_size = _find_entry(value, "Root Entry")
    root = _regular_chain(value, root_start)[:root_size]
    mini_fat_start = struct.unpack_from("<I", value, 60)[0]
    mini_fat = _regular_chain(value, mini_fat_start)
    chunks: list[bytes] = []
    while start != END:
        chunks.append(root[start * 64 : (start + 1) * 64])
        start = struct.unpack_from("<I", mini_fat, start * 4)[0]
    return b"".join(chunks)[:size]


def _regular_chain(value: bytes, start: int) -> bytes:
    fat = value[-512:]
    chunks: list[bytes] = []
    while start != END:
        chunks.append(value[(start + 1) * 512 : (start + 2) * 512])
        start = struct.unpack_from("<I", fat, start * 4)[0]
    return b"".join(chunks)
