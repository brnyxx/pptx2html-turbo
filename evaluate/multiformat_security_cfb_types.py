from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True, slots=True)
class CfbHeader:
    sector_size: int
    sector_count: int
    first_directory: int
    first_mini_fat: int
    mini_fat_count: int
    fat_sector_ids: tuple[int, ...]


@dataclass(frozen=True, slots=True)
class CfbEntry:
    name: str
    object_type: int
    left: int
    right: int
    child: int
    start_sector: int
    stream_size: int


@dataclass(frozen=True, slots=True)
class CfbStructure:
    header: CfbHeader
    fat: tuple[int, ...]
    entries: tuple[CfbEntry, ...]
