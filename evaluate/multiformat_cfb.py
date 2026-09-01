from __future__ import annotations

import io
import struct
from dataclasses import dataclass
from pathlib import Path
from typing import BinaryIO, Final

from evaluate.multiformat_package_validation import MAX_SOURCE_BYTES

CFBF_MAGIC: Final[bytes] = bytes.fromhex("d0cf11e0a1b11ae1")
FREE_SECTOR: Final[int] = 0xFFFFFFFF
END_OF_CHAIN: Final[int] = 0xFFFFFFFE
FAT_SECTOR: Final[int] = 0xFFFFFFFD
MAX_FAT_SECTORS: Final[int] = 4096
MAX_DIRECTORY_BYTES: Final[int] = 16 * 1024 * 1024


@dataclass(frozen=True, slots=True)
class _DirectoryEntry:
    name: str
    object_type: int
    left: int
    right: int
    child: int
    start_sector: int
    stream_size: int


def cfb_root_streams(path: Path) -> dict[str, int] | None:
    try:
        size = path.stat().st_size
        if not 0 < size <= MAX_SOURCE_BYTES:
            return None
        with path.open("rb") as source:
            return _cfb_root_streams(source, size)
    except (OSError, struct.error):
        return None


def cfb_root_streams_bytes(value: bytes) -> dict[str, int] | None:
    if not 0 < len(value) <= MAX_SOURCE_BYTES:
        return None
    try:
        with io.BytesIO(value) as source:
            return _cfb_root_streams(source, len(value))
    except (OSError, struct.error):
        return None


def _cfb_root_streams(source: BinaryIO, size: int) -> dict[str, int] | None:
    header = source.read(512)
    layout = _parse_header(header, size)
    if layout is None:
        return None
    sector_size, sector_count, fat_count, first_directory = layout
    fat_sectors = _fat_sector_ids(
        source,
        header,
        sector_size,
        sector_count,
        fat_count,
    )
    if fat_sectors is None:
        return None
    fat = _read_fat(source, fat_sectors, sector_size, sector_count)
    if fat is None:
        return None
    directory = _read_chain(
        source,
        first_directory,
        fat,
        sector_size,
        sector_count,
    )
    if directory is None:
        return None
    return _root_streams(directory)


def _parse_header(
    header: bytes,
    file_size: int,
) -> tuple[int, int, int, int] | None:
    if len(header) != 512 or header[:8] != CFBF_MAGIC:
        return None
    byte_order, sector_shift = struct.unpack_from("<HH", header, 28)
    major_version = struct.unpack_from("<H", header, 26)[0]
    if byte_order != 0xFFFE or sector_shift not in {9, 12}:
        return None
    if (major_version, sector_shift) not in {(3, 9), (4, 12)}:
        return None
    sector_size = 1 << sector_shift
    if file_size < sector_size * 2 or file_size % sector_size != 0:
        return None
    sector_count = file_size // sector_size - 1
    fat_count = struct.unpack_from("<I", header, 44)[0]
    first_directory = struct.unpack_from("<I", header, 48)[0]
    if not 0 < fat_count <= min(MAX_FAT_SECTORS, sector_count):
        return None
    if first_directory >= sector_count:
        return None
    return sector_size, sector_count, fat_count, first_directory


def _fat_sector_ids(
    source: BinaryIO,
    header: bytes,
    sector_size: int,
    sector_count: int,
    fat_count: int,
) -> list[int] | None:
    ids = [
        value
        for value in struct.unpack_from("<109I", header, 76)
        if value != FREE_SECTOR
    ]
    next_difat, difat_count = struct.unpack_from("<II", header, 68)
    visited: set[int] = set()
    for _ in range(difat_count):
        if next_difat >= sector_count or next_difat in visited:
            return None
        visited.add(next_difat)
        sector = _read_sector(source, next_difat, sector_size)
        values = struct.unpack(f"<{sector_size // 4}I", sector)
        ids.extend(value for value in values[:-1] if value != FREE_SECTOR)
        next_difat = values[-1]
    if next_difat not in {END_OF_CHAIN, FREE_SECTOR}:
        return None
    if len(ids) < fat_count:
        return None
    result = ids[:fat_count]
    if len(set(result)) != len(result) or any(
        value >= sector_count for value in result
    ):
        return None
    return result


def _read_fat(
    source: BinaryIO,
    fat_sectors: list[int],
    sector_size: int,
    sector_count: int,
) -> list[int] | None:
    values: list[int] = []
    for sector_id in fat_sectors:
        sector = _read_sector(source, sector_id, sector_size)
        values.extend(struct.unpack(f"<{sector_size // 4}I", sector))
    if len(values) < sector_count:
        return None
    return values[:sector_count]


def _read_chain(
    source: BinaryIO,
    first_sector: int,
    fat: list[int],
    sector_size: int,
    sector_count: int,
) -> bytes | None:
    chunks: list[bytes] = []
    visited: set[int] = set()
    current = first_sector
    while current != END_OF_CHAIN:
        if current >= sector_count or current in visited:
            return None
        visited.add(current)
        if len(chunks) * sector_size >= MAX_DIRECTORY_BYTES:
            return None
        chunks.append(_read_sector(source, current, sector_size))
        current = fat[current]
        if current in {FREE_SECTOR, FAT_SECTOR}:
            return None
    return b"".join(chunks)


def _read_sector(source: BinaryIO, sector_id: int, sector_size: int) -> bytes:
    source.seek((sector_id + 1) * sector_size)
    value = source.read(sector_size)
    if len(value) != sector_size:
        raise OSError("truncated CFBF sector")
    return value


def _root_streams(directory: bytes) -> dict[str, int] | None:
    entries: list[_DirectoryEntry | None] = []
    for offset in range(0, len(directory), 128):
        entry = directory[offset : offset + 128]
        if len(entry) != 128:
            return None
        name_length = struct.unpack_from("<H", entry, 64)[0]
        object_type = entry[66]
        if object_type == 0:
            entries.append(None)
            continue
        if object_type not in {1, 2, 5} or not 2 <= name_length <= 64:
            return None
        try:
            name = entry[: name_length - 2].decode("utf-16le")
        except UnicodeDecodeError:
            return None
        if not name:
            return None
        left, right, child = struct.unpack_from("<III", entry, 68)
        start_sector = struct.unpack_from("<I", entry, 116)[0]
        stream_size = struct.unpack_from("<Q", entry, 120)[0]
        entries.append(
            _DirectoryEntry(
                name,
                object_type,
                left,
                right,
                child,
                start_sector,
                stream_size,
            )
        )
    if not entries or entries[0] is None or entries[0].object_type != 5:
        return None
    streams: dict[str, int] = {}
    visited: set[int] = set()
    if not _collect_root_streams(entries, entries[0].child, visited, streams):
        return None
    return streams


def _collect_root_streams(
    entries: list[_DirectoryEntry | None],
    index: int,
    visited: set[int],
    streams: dict[str, int],
) -> bool:
    if index == FREE_SECTOR:
        return True
    if index >= len(entries) or index in visited:
        return False
    entry = entries[index]
    if entry is None or entry.object_type == 5:
        return False
    visited.add(index)
    if not _collect_root_streams(entries, entry.left, visited, streams):
        return False
    if entry.object_type == 2:
        if entry.name in streams:
            return False
        if entry.stream_size > 0 and entry.start_sector in {
            FREE_SECTOR,
            END_OF_CHAIN,
            FAT_SECTOR,
        }:
            return False
        streams[entry.name] = entry.stream_size
    return _collect_root_streams(entries, entry.right, visited, streams)
