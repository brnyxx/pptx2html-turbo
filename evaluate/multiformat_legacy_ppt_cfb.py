from __future__ import annotations

import struct
from dataclasses import dataclass
from typing import Final

from evaluate.multiformat_cfb import END_OF_CHAIN, FAT_SECTOR, FREE_SECTOR
from evaluate.multiformat_security_cfb_parser import parse_cfb_structure, read_mini_fat
from evaluate.multiformat_security_cfb_streams import (
    MINI_STREAM_CUTOFF,
    has_directory_cycle,
    root_sibling_indices,
)
from evaluate.multiformat_security_cfb_types import CfbEntry, CfbStructure

_INVALID_SECTORS: Final[frozenset[int]] = frozenset(
    {FREE_SECTOR, END_OF_CHAIN, FAT_SECTOR}
)


class LegacyPptCfbError(ValueError):
    pass


@dataclass(frozen=True, slots=True)
class _LocatedEntry:
    index: int
    entry: CfbEntry


class MutableCfb:
    _value: bytearray
    structure: CfbStructure
    _directory_sectors: tuple[int, ...]

    def __init__(self, value: bytes) -> None:
        structure, directory_cycle = parse_cfb_structure(value)
        if structure is None or directory_cycle:
            raise LegacyPptCfbError("malformed CFB")
        if struct.unpack_from("<I", value, 72)[0] != 0:
            raise LegacyPptCfbError("DIFAT CFB is unsupported")
        if has_directory_cycle(structure.entries):
            raise LegacyPptCfbError("cyclic CFB directory")
        self._value = bytearray(value)
        self.structure = structure
        self._directory_sectors = self._chain(structure.header.first_directory)
        required = len(structure.entries) * 128
        if len(self._directory_sectors) * structure.header.sector_size < required:
            raise LegacyPptCfbError("truncated CFB directory")

    def bytes(self) -> bytes:
        return bytes(self._value)

    def has_root_stream(self, name: str) -> bool:
        return self._root_entry(name, required=False) is not None

    def read_root_stream(self, name: str) -> bytes:
        located = self._root_entry(name)
        if located is None:
            raise LegacyPptCfbError(f"missing CFB stream: {name}")
        return self._read_entry(located.entry)

    def replace_root_stream(self, name: str, content: bytes) -> None:
        located = self._root_entry(name)
        if located is None:
            raise LegacyPptCfbError(f"missing CFB stream: {name}")
        entry = located.entry
        if entry.stream_size < MINI_STREAM_CUTOFF:
            if len(content) != entry.stream_size:
                raise LegacyPptCfbError("mini stream size change is unsupported")
            self._write_mini(entry, content)
        else:
            if len(content) < MINI_STREAM_CUTOFF:
                raise LegacyPptCfbError("regular stream crossed mini-stream cutoff")
            self._write_regular(entry, content)
            self._write_directory_size(located.index, len(content))

    def replace_mini_stream(self, name: str, content: bytes) -> None:
        located = self._root_entry(name)
        if located is None:
            raise LegacyPptCfbError(f"missing CFB stream: {name}")
        if located.entry.stream_size >= MINI_STREAM_CUTOFF:
            raise LegacyPptCfbError("expected a mini stream")
        if len(content) != located.entry.stream_size:
            raise LegacyPptCfbError("mini stream size changed")
        self._write_mini(located.entry, content)

    def _root_entry(
        self,
        name: str,
        *,
        required: bool = True,
    ) -> _LocatedEntry | None:
        matches = [
            _LocatedEntry(index, self.structure.entries[index])
            for index in root_sibling_indices(self.structure.entries)
            if self.structure.entries[index].object_type == 2
            and self.structure.entries[index].name == name
        ]
        if len(matches) > 1 or (required and not matches):
            raise LegacyPptCfbError(f"ambiguous or missing CFB stream: {name}")
        return matches[0] if matches else None

    def _read_entry(self, entry: CfbEntry) -> bytes:
        if entry.stream_size == 0:
            return b""
        if entry.object_type != 5 and entry.stream_size < MINI_STREAM_CUTOFF:
            return self._read_mini(entry)
        chain = self._chain(entry.start_sector)
        capacity = len(chain) * self.structure.header.sector_size
        if entry.stream_size > capacity:
            raise LegacyPptCfbError("truncated regular stream")
        return b"".join(self._sector(sector) for sector in chain)[: entry.stream_size]

    def _read_mini(self, entry: CfbEntry) -> bytes:
        mini_fat = read_mini_fat(bytes(self._value), self.structure)
        root = self.structure.entries[0]
        if mini_fat is None or root.object_type != 5:
            raise LegacyPptCfbError("missing mini FAT")
        root_content = self._read_entry(root)
        chain = self._mini_chain(entry.start_sector, mini_fat, len(root_content))
        content = b"".join(
            root_content[index * 64 : (index + 1) * 64] for index in chain
        )
        if len(content) < entry.stream_size:
            raise LegacyPptCfbError("truncated mini stream")
        return content[: entry.stream_size]

    def _write_regular(self, entry: CfbEntry, content: bytes) -> None:
        chain = self._chain(entry.start_sector)
        sector_size = self.structure.header.sector_size
        if len(content) > len(chain) * sector_size:
            raise LegacyPptCfbError("regular stream capacity overflow")
        padded = content.ljust(len(chain) * sector_size, b"\x00")
        for ordinal, sector in enumerate(chain):
            start = self._sector_offset(sector)
            chunk = padded[ordinal * sector_size : (ordinal + 1) * sector_size]
            self._value[start : start + sector_size] = chunk

    def _write_mini(self, entry: CfbEntry, content: bytes) -> None:
        mini_fat = read_mini_fat(bytes(self._value), self.structure)
        root = self.structure.entries[0]
        if mini_fat is None or root.object_type != 5:
            raise LegacyPptCfbError("missing mini FAT")
        root_content = bytearray(self._read_entry(root))
        chain = self._mini_chain(entry.start_sector, mini_fat, len(root_content))
        if len(content) > len(chain) * 64:
            raise LegacyPptCfbError("mini stream capacity overflow")
        remaining = memoryview(content)
        for index in chain:
            count = min(64, len(remaining))
            if count:
                root_content[index * 64 : index * 64 + count] = remaining[:count]
                remaining = remaining[count:]
        self._write_regular(root, bytes(root_content))

    def _chain(self, start: int) -> tuple[int, ...]:
        result: list[int] = []
        visited: set[int] = set()
        current = start
        while current != END_OF_CHAIN:
            if (
                current in visited
                or current >= self.structure.header.sector_count
                or current in _INVALID_SECTORS
            ):
                raise LegacyPptCfbError("invalid or cyclic FAT chain")
            visited.add(current)
            result.append(current)
            current = self.structure.fat[current]
        return tuple(result)

    def _mini_chain(
        self,
        start: int,
        mini_fat: tuple[int, ...],
        root_size: int,
    ) -> tuple[int, ...]:
        result: list[int] = []
        visited: set[int] = set()
        current = start
        while current != END_OF_CHAIN:
            if (
                current in visited
                or current >= len(mini_fat)
                or (current + 1) * 64 > root_size
                or current in _INVALID_SECTORS
            ):
                raise LegacyPptCfbError("invalid or cyclic mini FAT chain")
            visited.add(current)
            result.append(current)
            current = mini_fat[current]
        return tuple(result)

    def _write_directory_size(self, index: int, size: int) -> None:
        sector_size = self.structure.header.sector_size
        byte_offset = index * 128 + 120
        sector = self._directory_sectors[byte_offset // sector_size]
        target = self._sector_offset(sector) + byte_offset % sector_size
        struct.pack_into("<Q", self._value, target, size)

    def _sector(self, sector: int) -> bytes:
        start = self._sector_offset(sector)
        size = self.structure.header.sector_size
        return bytes(self._value[start : start + size])

    def _sector_offset(self, sector: int) -> int:
        return (sector + 1) * self.structure.header.sector_size
