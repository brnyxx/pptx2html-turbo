from __future__ import annotations

import struct
from enum import StrEnum
from pathlib import Path
from typing import assert_never

from evaluate.multiformat_package_validation import MAX_SOURCE_BYTES
from evaluate.multiformat_source_fixture import (
    END_OF_CHAIN,
    FAT_SECTOR,
    FREE_SECTOR,
    SourceFixtureError,
    write_positive_source,
)

DIRECTORY_OFFSET = 512
ROOT_ENTRY_OFFSET = DIRECTORY_OFFSET
PRIMARY_ENTRY_OFFSET = DIRECTORY_OFFSET + 128
SEMANTIC_ENTRY_OFFSET = DIRECTORY_OFFSET + 256
CHILD_ENTRY_OFFSET = DIRECTORY_OFFSET + 384
FAT_OFFSET = 5120
LINK_STREAM_OFFSET = FAT_OFFSET


class CfbSecurityFamily(StrEnum):
    MALFORMED_CFBF = "malformed-cfbf"
    FAT_CYCLE = "fat-cycle"
    DIFAT_OVERFLOW = "difat-overflow"
    DIRECTORY_CYCLE = "directory-cycle"
    MINI_STREAM_CORRUPTION = "mini-stream-corruption"
    TRUNCATED_STREAM = "truncated-stream"
    EXTERNAL_LINK = "external-link"
    MACRO_STORAGE = "macro-storage"
    EMBEDDED_OBJECT = "embedded-object"
    OVERSIZED_STREAM = "oversized-stream"


def write_cfb_security_fixture(
    path: Path,
    document_format: str,
    family: str,
) -> None:
    try:
        parsed_family = CfbSecurityFamily(family)
    except ValueError as error:
        raise SourceFixtureError(
            f"unsupported CFB security family: {family}"
        ) from error
    write_positive_source(path, document_format, family)
    value = bytearray(path.read_bytes())
    match parsed_family:
        case CfbSecurityFamily.MALFORMED_CFBF:
            value[0] ^= 0xFF
        case CfbSecurityFamily.FAT_CYCLE:
            struct.pack_into("<I", value, FAT_OFFSET + 8 * 4, 1)
        case CfbSecurityFamily.DIFAT_OVERFLOW:
            struct.pack_into("<II", value, 68, 999, 1)
        case CfbSecurityFamily.DIRECTORY_CYCLE:
            struct.pack_into("<I", value, PRIMARY_ENTRY_OFFSET + 72, 1)
        case CfbSecurityFamily.MINI_STREAM_CORRUPTION:
            value[SEMANTIC_ENTRY_OFFSET : SEMANTIC_ENTRY_OFFSET + 128] = (
                _directory_entry(
                    "MiniBroken",
                    2,
                    start_sector=999,
                    stream_size=64,
                )
            )
        case CfbSecurityFamily.TRUNCATED_STREAM:
            struct.pack_into("<Q", value, PRIMARY_ENTRY_OFFSET + 120, 8192)
        case CfbSecurityFamily.EXTERNAL_LINK:
            value[FAT_OFFSET:FAT_OFFSET] = b"\x00" * 4096
            struct.pack_into("<I", value, 76, 17)
            value[SEMANTIC_ENTRY_OFFSET : SEMANTIC_ENTRY_OFFSET + 128] = (
                _directory_entry(
                    "LinkInfo",
                    2,
                    start_sector=9,
                    stream_size=4096,
                )
            )
            target = b"https://example.invalid/security-link"
            value[LINK_STREAM_OFFSET : LINK_STREAM_OFFSET + len(target)] = target
            moved_fat = FAT_OFFSET + 4096
            for sector_id in range(9, 16):
                struct.pack_into(
                    "<I",
                    value,
                    moved_fat + sector_id * 4,
                    sector_id + 1,
                )
            struct.pack_into("<I", value, moved_fat + 16 * 4, END_OF_CHAIN)
            struct.pack_into("<I", value, moved_fat + 17 * 4, FAT_SECTOR)
        case CfbSecurityFamily.MACRO_STORAGE:
            value[SEMANTIC_ENTRY_OFFSET : SEMANTIC_ENTRY_OFFSET + 128] = (
                _directory_entry("VBA", 1, child=3)
            )
            value[CHILD_ENTRY_OFFSET : CHILD_ENTRY_OFFSET + 128] = _directory_entry(
                "_VBA_PROJECT",
                2,
            )
        case CfbSecurityFamily.EMBEDDED_OBJECT:
            value[SEMANTIC_ENTRY_OFFSET : SEMANTIC_ENTRY_OFFSET + 128] = (
                _directory_entry("ObjectPool", 1, child=3)
            )
            value[CHILD_ENTRY_OFFSET : CHILD_ENTRY_OFFSET + 128] = _directory_entry(
                "\x01Ole10Native",
                2,
            )
        case CfbSecurityFamily.OVERSIZED_STREAM:
            struct.pack_into(
                "<Q",
                value,
                PRIMARY_ENTRY_OFFSET + 120,
                MAX_SOURCE_BYTES + 1,
            )
        case unreachable:
            assert_never(unreachable)
    path.write_bytes(value)


def _directory_entry(
    name: str,
    object_type: int,
    *,
    child: int = FREE_SECTOR,
    right: int = FREE_SECTOR,
    start_sector: int = END_OF_CHAIN,
    stream_size: int = 0,
) -> bytes:
    entry = bytearray(128)
    encoded = name.encode("utf-16le") + b"\x00\x00"
    entry[: len(encoded)] = encoded
    struct.pack_into("<H", entry, 64, len(encoded))
    entry[66] = object_type
    entry[67] = 1
    struct.pack_into("<III", entry, 68, FREE_SECTOR, right, child)
    struct.pack_into("<I", entry, 116, start_sector)
    struct.pack_into("<Q", entry, 120, stream_size)
    return bytes(entry)
