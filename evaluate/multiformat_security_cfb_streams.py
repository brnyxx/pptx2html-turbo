from __future__ import annotations

import math
from typing import Final

from evaluate.multiformat_cfb import END_OF_CHAIN, FAT_SECTOR, FREE_SECTOR
from evaluate.multiformat_package_validation import MAX_SOURCE_BYTES
from evaluate.multiformat_security_cfb_parser import (
    chain_length,
    read_mini_fat,
    read_regular_stream,
)
from evaluate.multiformat_security_cfb_types import CfbEntry, CfbStructure

MINI_STREAM_CUTOFF: Final[int] = 4_096
MAX_LINK_STREAM_BYTES: Final[int] = 1024 * 1024
URL_MARKERS: Final[tuple[bytes, ...]] = (b"http://", b"https://", b"file://")


def has_regular_fat_cycle(structure: CfbStructure) -> bool:
    for index in root_tree_indices(structure.entries):
        entry = structure.entries[index]
        if entry.object_type != 2 or entry.stream_size < MINI_STREAM_CUTOFF:
            continue
        _, cyclic = chain_length(
            entry.start_sector,
            structure.fat,
            structure.header.sector_count,
        )
        if cyclic:
            return True
    return False


def has_directory_cycle(entries: tuple[CfbEntry, ...]) -> bool:
    active: set[int] = set()
    visited: set[int] = set()
    pending = [(0, False)] if entries else []
    while pending:
        index, exiting = pending.pop()
        if index == FREE_SECTOR:
            continue
        if index >= len(entries) or entries[index].object_type == 0:
            continue
        if exiting:
            active.remove(index)
            visited.add(index)
            continue
        if index in active:
            return True
        if index in visited:
            continue
        active.add(index)
        entry = entries[index]
        pending.append((index, True))
        pending.extend(
            (child, False) for child in (entry.left, entry.right, entry.child)
        )
    return False


def has_mini_stream_corruption(
    value: bytes,
    structure: CfbStructure,
) -> bool:
    entries = structure.entries
    header = structure.header
    mini_streams = [
        entries[index]
        for index in root_tree_indices(entries)
        if entries[index].object_type == 2
        and 0 < entries[index].stream_size < MINI_STREAM_CUTOFF
    ]
    if not mini_streams:
        return False
    root = entries[0]
    if (
        header.mini_fat_count == 0
        or header.first_mini_fat in {FREE_SECTOR, END_OF_CHAIN, FAT_SECTOR}
        or root.object_type != 5
        or root.stream_size == 0
        or root.start_sector in {FREE_SECTOR, END_OF_CHAIN, FAT_SECTOR}
    ):
        return True
    mini_fat = read_mini_fat(value, structure)
    mini_stream = read_regular_stream(
        value,
        root,
        structure,
        MAX_SOURCE_BYTES,
    )
    if mini_fat is None or mini_stream is None:
        return True
    for entry in mini_streams:
        required = math.ceil(entry.stream_size / 64)
        visited: set[int] = set()
        current = entry.start_sector
        while current != END_OF_CHAIN and len(visited) < required:
            if (
                current >= len(mini_fat)
                or current in visited
                or (current + 1) * 64 > len(mini_stream)
            ):
                return True
            visited.add(current)
            current = mini_fat[current]
        if len(visited) < required:
            return True
    return False


def has_truncated_stream(structure: CfbStructure) -> bool:
    for index in root_tree_indices(structure.entries):
        entry = structure.entries[index]
        if (
            entry.object_type != 2
            or entry.stream_size < MINI_STREAM_CUTOFF
            or entry.stream_size > MAX_SOURCE_BYTES
        ):
            continue
        sectors, cyclic = chain_length(
            entry.start_sector,
            structure.fat,
            structure.header.sector_count,
        )
        required = math.ceil(entry.stream_size / structure.header.sector_size)
        if not cyclic and sectors < required:
            return True
    return False


def has_external_link(value: bytes, structure: CfbStructure) -> bool:
    for index in root_tree_indices(structure.entries):
        entry = structure.entries[index]
        if entry.object_type != 2 or entry.name.casefold() != "linkinfo":
            continue
        content = read_regular_stream(
            value,
            entry,
            structure,
            MAX_LINK_STREAM_BYTES,
        )
        if content is not None and any(marker in content for marker in URL_MARKERS):
            return True
    return False


def storage_has_descendant(
    entries: tuple[CfbEntry, ...],
    storage_name: str,
    descendant_name: str,
) -> bool:
    for index in root_tree_indices(entries):
        entry = entries[index]
        if entry.object_type != 1 or entry.name.casefold() != storage_name:
            continue
        pending = [entry.child]
        visited: set[int] = set()
        while pending:
            index = pending.pop()
            if (
                index == FREE_SECTOR
                or index >= len(entries)
                or index in visited
                or entries[index].object_type == 0
            ):
                continue
            visited.add(index)
            child = entries[index]
            if child.name.casefold() == descendant_name:
                return True
            pending.extend((child.left, child.right, child.child))
    return False


def has_embedded_storage(entries: tuple[CfbEntry, ...]) -> bool:
    storage_names = {
        entries[index].name.casefold()
        for index in root_tree_indices(entries)
        if entries[index].object_type == 1
        and (
            entries[index].name.casefold() == "objectpool"
            or entries[index].name.casefold().startswith("mbd")
        )
    }
    return any(
        storage_has_descendant(entries, name, "\x01ole10native")
        for name in storage_names
    )


def root_tree_indices(entries: tuple[CfbEntry, ...]) -> frozenset[int]:
    if not entries or entries[0].object_type != 5:
        return frozenset()
    pending = [entries[0].child]
    visited: set[int] = set()
    while pending:
        index = pending.pop()
        if (
            index == FREE_SECTOR
            or index >= len(entries)
            or index in visited
            or entries[index].object_type == 0
        ):
            continue
        visited.add(index)
        entry = entries[index]
        pending.extend((entry.left, entry.right))
        if entry.object_type == 1:
            pending.append(entry.child)
    return frozenset(visited)


def root_sibling_indices(entries: tuple[CfbEntry, ...]) -> frozenset[int]:
    if not entries or entries[0].object_type != 5:
        return frozenset()
    pending = [entries[0].child]
    visited: set[int] = set()
    while pending:
        index = pending.pop()
        if (
            index == FREE_SECTOR
            or index >= len(entries)
            or index in visited
            or entries[index].object_type == 0
        ):
            continue
        visited.add(index)
        pending.extend((entries[index].left, entries[index].right))
    return frozenset(visited)
