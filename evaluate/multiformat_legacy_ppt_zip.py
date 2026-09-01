from __future__ import annotations

import struct
from dataclasses import dataclass
from typing import Final

_LOCAL: Final[int] = 0x04034B50
_CENTRAL: Final[int] = 0x02014B50
_EOCD: Final[bytes] = b"PK\x05\x06"
_ZIP64_EXTRA: Final[int] = 0x0001
_ODF_CHART_MIME: Final[bytes] = b"application/vnd.oasis.opendocument.chart"
_CANONICAL_TIME: Final[int] = 0
_CANONICAL_DATE: Final[int] = 0x0021


class LegacyPptZipError(ValueError):
    pass


@dataclass(frozen=True, slots=True)
class _Member:
    name: bytes
    flags: int
    method: int
    crc: int
    compressed_size: int
    size: int
    local_offset: int
    central_offset: int


def canonicalize_odf_zip_timestamps(value: bytes) -> bytes:
    members, central_start = _parse(value)
    if not _is_odf_chart(value, members, central_start):
        return value
    result = bytearray(value)
    for member in members:
        struct.pack_into(
            "<HH", result, member.local_offset + 10, _CANONICAL_TIME, _CANONICAL_DATE
        )
        struct.pack_into(
            "<HH", result, member.central_offset + 12, _CANONICAL_TIME, _CANONICAL_DATE
        )
    return bytes(result)


def _parse(value: bytes) -> tuple[tuple[_Member, ...], int]:
    candidates = [
        offset
        for offset in range(max(0, len(value) - 65_557), len(value) - 3)
        if value[offset : offset + 4] == _EOCD
    ]
    valid = [offset for offset in candidates if _valid_eocd_end(value, offset)]
    if len(valid) != 1:
        raise LegacyPptZipError("ambiguous or missing ZIP end record")
    eocd = valid[0]
    disk, central_disk, disk_count, total_count, central_size, central_start, _ = (
        struct.unpack_from("<HHHHIIH", value, eocd + 4)
    )
    if disk != 0 or central_disk != 0 or disk_count != total_count:
        raise LegacyPptZipError("multidisk ZIP is unsupported")
    if 0xFFFF in {disk_count, total_count} or 0xFFFFFFFF in {
        central_size,
        central_start,
    }:
        raise LegacyPptZipError("Zip64 is unsupported")
    if central_start + central_size != eocd:
        raise LegacyPptZipError("noncontiguous ZIP central directory")
    members: list[_Member] = []
    offset = central_start
    for _ in range(total_count):
        member, offset = _parse_central(value, offset, eocd)
        members.append(member)
    if offset != eocd or not members:
        raise LegacyPptZipError("malformed ZIP central directory")
    _validate_locals(value, members, central_start)
    return tuple(members), central_start


def _valid_eocd_end(value: bytes, offset: int) -> bool:
    if offset + 22 > len(value):
        return False
    comment_size = struct.unpack_from("<H", value, offset + 20)[0]
    return offset + 22 + comment_size == len(value)


def _parse_central(value: bytes, offset: int, end: int) -> tuple[_Member, int]:
    if offset + 46 > end or struct.unpack_from("<I", value, offset)[0] != _CENTRAL:
        raise LegacyPptZipError("malformed ZIP central entry")
    (
        _,
        made_by,
        needed,
        flags,
        method,
        _,
        _,
        crc,
        compressed_size,
        size,
        name_size,
        extra_size,
        comment_size,
        disk,
        _,
        _,
        local_offset,
    ) = struct.unpack_from("<IHHHHHHIIIHHHHHII", value, offset)
    finish = offset + 46 + name_size + extra_size + comment_size
    if finish > end or disk != 0 or needed >= 45 or made_by & 0xFF >= 45:
        raise LegacyPptZipError("unsupported ZIP central entry")
    if 0xFFFFFFFF in {compressed_size, size, local_offset}:
        raise LegacyPptZipError("Zip64 is unsupported")
    name = value[offset + 46 : offset + 46 + name_size]
    extra = value[offset + 46 + name_size : offset + 46 + name_size + extra_size]
    _reject_zip64_extra(extra)
    return (
        _Member(name, flags, method, crc, compressed_size, size, local_offset, offset),
        finish,
    )


def _validate_locals(value: bytes, members: list[_Member], central_start: int) -> None:
    seen: set[int] = set()
    spans: list[tuple[int, int]] = []
    for member in members:
        offset = member.local_offset
        if offset in seen or offset + 30 > central_start:
            raise LegacyPptZipError("ambiguous ZIP local entry")
        seen.add(offset)
        (
            signature,
            needed,
            flags,
            method,
            _,
            _,
            crc,
            compressed_size,
            size,
            name_size,
            extra_size,
        ) = struct.unpack_from("<IHHHHHIIIHH", value, offset)
        header_end = offset + 30 + name_size + extra_size
        data_end = header_end + member.compressed_size
        if (
            signature != _LOCAL
            or needed >= 45
            or flags != member.flags
            or method != member.method
            or flags & 0x0001
            or header_end > central_start
            or data_end > central_start
            or value[offset + 30 : offset + 30 + name_size] != member.name
        ):
            raise LegacyPptZipError("malformed ZIP local entry")
        if not flags & 0x0008 and (
            crc != member.crc
            or compressed_size != member.compressed_size
            or size != member.size
        ):
            raise LegacyPptZipError("inconsistent ZIP member metadata")
        extra = value[offset + 30 + name_size : header_end]
        _reject_zip64_extra(extra)
        spans.append((offset, data_end))
    spans.sort()
    if spans[0][0] != 0 or any(
        left[1] > right[0] for left, right in zip(spans, spans[1:])
    ):
        raise LegacyPptZipError("overlapping ZIP local entries")


def _reject_zip64_extra(extra: bytes) -> None:
    offset = 0
    while offset < len(extra):
        if offset + 4 > len(extra):
            raise LegacyPptZipError("malformed ZIP extra field")
        kind, size = struct.unpack_from("<HH", extra, offset)
        offset += 4
        if offset + size > len(extra):
            raise LegacyPptZipError("malformed ZIP extra field")
        if kind == _ZIP64_EXTRA:
            raise LegacyPptZipError("Zip64 is unsupported")
        offset += size


def _is_odf_chart(
    value: bytes, members: tuple[_Member, ...], central_start: int
) -> bool:
    matches = [member for member in members if member.name == b"mimetype"]
    if len(matches) != 1:
        return False
    member = matches[0]
    if (
        member.method != 0
        or member.flags & 0x0008
        or member.size != len(_ODF_CHART_MIME)
    ):
        return False
    name_size, extra_size = struct.unpack_from("<HH", value, member.local_offset + 26)
    start = member.local_offset + 30 + name_size + extra_size
    return (
        start + member.size <= central_start
        and value[start : start + member.size] == _ODF_CHART_MIME
    )
