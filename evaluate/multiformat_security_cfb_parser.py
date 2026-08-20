from __future__ import annotations

import struct
from typing import Final

from evaluate.multiformat_cfb import (
    END_OF_CHAIN,
    FAT_SECTOR,
    FREE_SECTOR,
    MAX_DIRECTORY_BYTES,
)
from evaluate.multiformat_security_cfb_types import (
    CfbEntry,
    CfbHeader,
    CfbStructure,
)

MAX_FAT_SECTORS: Final[int] = 4_096


def plausible_cfb_header(value: bytes) -> bool:
    if len(value) < 512:
        return False
    try:
        major, byte_order, sector_shift = struct.unpack_from("<HHH", value, 26)
    except struct.error:
        return False
    return (
        byte_order == 0xFFFE
        and (major, sector_shift) in {(3, 9), (4, 12)}
        and len(value) % (1 << sector_shift) == 0
    )


def has_difat_overflow(value: bytes) -> bool:
    if len(value) < 512:
        return False
    try:
        sector_shift = struct.unpack_from("<H", value, 30)[0]
        next_difat, difat_count = struct.unpack_from("<II", value, 68)
    except struct.error:
        return False
    if sector_shift not in {9, 12}:
        return False
    sector_count = len(value) // (1 << sector_shift) - 1
    return difat_count > sector_count or (
        difat_count > 0 and next_difat >= sector_count
    )


def parse_cfb_structure(value: bytes) -> tuple[CfbStructure | None, bool]:
    header = _parse_header(value)
    if header is None:
        return None, False
    fat = _read_fat(value, header)
    if fat is None:
        return None, False
    directory, directory_cycle = _read_chain(
        value,
        header.first_directory,
        fat,
        header,
        MAX_DIRECTORY_BYTES,
    )
    if directory is None:
        return None, directory_cycle
    entries = _parse_entries(directory)
    if entries is None:
        return None, False
    return CfbStructure(header, fat, entries), False


def chain_length(
    start: int,
    fat: tuple[int, ...],
    sector_count: int,
) -> tuple[int, bool]:
    visited: set[int] = set()
    current = start
    while current != END_OF_CHAIN:
        if current in visited:
            return len(visited), True
        if current >= sector_count or current in {FREE_SECTOR, FAT_SECTOR}:
            return len(visited), False
        visited.add(current)
        current = fat[current]
    return len(visited), False


def read_regular_stream(
    value: bytes,
    entry: CfbEntry,
    structure: CfbStructure,
    byte_limit: int,
) -> bytes | None:
    if entry.stream_size > byte_limit:
        return None
    content, cyclic = _read_chain(
        value,
        entry.start_sector,
        structure.fat,
        structure.header,
        byte_limit,
    )
    if cyclic or content is None or len(content) < entry.stream_size:
        return None
    return content[: entry.stream_size]


def read_mini_fat(
    value: bytes,
    structure: CfbStructure,
) -> tuple[int, ...] | None:
    header = structure.header
    if header.mini_fat_count == 0 or header.mini_fat_count > header.sector_count:
        return None
    byte_count = header.mini_fat_count * header.sector_size
    content, cyclic = _read_chain(
        value,
        header.first_mini_fat,
        structure.fat,
        header,
        byte_count,
    )
    if cyclic or content is None or len(content) < byte_count:
        return None
    return struct.unpack(f"<{byte_count // 4}I", content[:byte_count])


def _parse_header(value: bytes) -> CfbHeader | None:
    if not plausible_cfb_header(value):
        return None
    sector_shift = struct.unpack_from("<H", value, 30)[0]
    sector_size = 1 << sector_shift
    sector_count = len(value) // sector_size - 1
    fat_count = struct.unpack_from("<I", value, 44)[0]
    first_directory = struct.unpack_from("<I", value, 48)[0]
    first_mini_fat, mini_fat_count = struct.unpack_from("<II", value, 60)
    fat_ids = tuple(
        item for item in struct.unpack_from("<109I", value, 76) if item != FREE_SECTOR
    )
    if (
        not 0 < fat_count <= min(MAX_FAT_SECTORS, sector_count)
        or len(fat_ids) < fat_count
        or first_directory >= sector_count
    ):
        return None
    selected = fat_ids[:fat_count]
    if len(set(selected)) != len(selected) or any(
        item >= sector_count for item in selected
    ):
        return None
    return CfbHeader(
        sector_size,
        sector_count,
        first_directory,
        first_mini_fat,
        mini_fat_count,
        selected,
    )


def _read_fat(value: bytes, header: CfbHeader) -> tuple[int, ...] | None:
    entries: list[int] = []
    for sector_id in header.fat_sector_ids:
        sector = _sector(value, sector_id, header)
        if sector is None:
            return None
        entries.extend(struct.unpack(f"<{header.sector_size // 4}I", sector))
    return (
        tuple(entries[: header.sector_count])
        if len(entries) >= header.sector_count
        else None
    )


def _sector(value: bytes, sector_id: int, header: CfbHeader) -> bytes | None:
    if sector_id >= header.sector_count:
        return None
    start = (sector_id + 1) * header.sector_size
    result = value[start : start + header.sector_size]
    return result if len(result) == header.sector_size else None


def _read_chain(
    value: bytes,
    start: int,
    fat: tuple[int, ...],
    header: CfbHeader,
    byte_limit: int,
) -> tuple[bytes | None, bool]:
    chunks: list[bytes] = []
    visited: set[int] = set()
    current = start
    while current != END_OF_CHAIN:
        if current in visited:
            return None, True
        if current >= header.sector_count or current in {FREE_SECTOR, FAT_SECTOR}:
            return None, False
        if len(chunks) * header.sector_size >= byte_limit:
            return None, False
        sector = _sector(value, current, header)
        if sector is None:
            return None, False
        visited.add(current)
        chunks.append(sector)
        current = fat[current]
    return b"".join(chunks), False


def _parse_entries(directory: bytes) -> tuple[CfbEntry, ...] | None:
    entries: list[CfbEntry] = []
    for offset in range(0, len(directory), 128):
        raw = directory[offset : offset + 128]
        if len(raw) != 128:
            return None
        name_length = struct.unpack_from("<H", raw, 64)[0]
        object_type = raw[66]
        if object_type == 0:
            entries.append(CfbEntry("", 0, 0, 0, 0, 0, 0))
            continue
        if object_type not in {1, 2, 5} or not 2 <= name_length <= 64:
            return None
        try:
            name = raw[: name_length - 2].decode("utf-16le")
        except UnicodeDecodeError:
            return None
        left, right, child = struct.unpack_from("<III", raw, 68)
        entries.append(
            CfbEntry(
                name,
                object_type,
                left,
                right,
                child,
                struct.unpack_from("<I", raw, 116)[0],
                struct.unpack_from("<Q", raw, 120)[0],
            )
        )
    return tuple(entries)
