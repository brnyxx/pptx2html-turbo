from __future__ import annotations

import struct
import zlib
from dataclasses import dataclass, replace
from typing import Final

from evaluate.multiformat_legacy_ppt_cfb import MutableCfb
from evaluate.multiformat_legacy_ppt_zip import canonicalize_odf_zip_timestamps

EX_OLE_OBJ_STG: Final[int] = 0x1011
PERSIST_DIRECTORY_ATOM: Final[int] = 0x1772
USER_EDIT_ATOM: Final[int] = 0x0FF5
CURRENT_USER_ATOM: Final[int] = 0x0FF6
_MAX_DECOMPRESSED_STORAGE: Final[int] = 64 * 1024 * 1024


class LegacyPptRecordError(ValueError):
    pass


@dataclass(frozen=True, slots=True)
class _Record:
    old_offset: int
    options: int
    kind: int
    body: bytes
    children: tuple[_Record, ...] | None = None

    @property
    def rebuilt_size(self) -> int:
        if self.children is None:
            return 8 + len(self.body)
        return 8 + sum(child.rebuilt_size for child in self.children)


def rebuild_power_point_stream(
    value: bytes,
) -> tuple[bytes, dict[int, int], dict[int, int]]:
    records = _parse_records(value, 0)
    transformed = tuple(_transform_tree(record) for record in records)
    old_to_new: dict[int, int] = {}
    kinds: dict[int, int] = {}
    _ = _assign_offsets(transformed, 0, old_to_new, kinds)
    result = b"".join(_serialize(record, old_to_new, kinds) for record in transformed)
    return result, old_to_new, kinds


def rewrite_current_user_stream(
    value: bytes,
    old_to_new: dict[int, int],
    kinds: dict[int, int],
) -> bytes:
    if len(value) < 20:
        raise LegacyPptRecordError("truncated Current User stream")
    _, kind, length = struct.unpack_from("<HHI", value)
    if kind != CURRENT_USER_ATOM or length + 8 != len(value):
        raise LegacyPptRecordError("malformed CurrentUserAtom")
    old_offset = struct.unpack_from("<I", value, 16)[0]
    new_offset = _mapped_offset(old_offset, old_to_new, kinds, USER_EDIT_ATOM, False)
    result = bytearray(value)
    struct.pack_into("<I", result, 16, new_offset)
    return bytes(result)


def _parse_records(
    value: bytes, base: int, *, allow_empty: bool = False
) -> tuple[_Record, ...]:
    records: list[_Record] = []
    offset = 0
    while offset < len(value):
        if offset + 8 > len(value):
            raise LegacyPptRecordError("truncated PowerPoint record header")
        options, kind, length = struct.unpack_from("<HHI", value, offset)
        end = offset + 8 + length
        if end > len(value):
            raise LegacyPptRecordError("truncated PowerPoint record")
        body = value[offset + 8 : end]
        children = (
            _parse_records(body, base + offset + 8, allow_empty=True)
            if kind != EX_OLE_OBJ_STG and options & 0x000F == 0x000F
            else None
        )
        records.append(_Record(base + offset, options, kind, body, children))
        offset = end
    if not records and not allow_empty:
        raise LegacyPptRecordError("empty PowerPoint record sequence")
    return tuple(records)


def _transform_tree(record: _Record) -> _Record:
    if record.kind == EX_OLE_OBJ_STG:
        rec_version = record.options & 0x000F
        rec_instance = record.options >> 4
        if rec_version != 0 or rec_instance not in {0, 1}:
            raise LegacyPptRecordError("unsupported ExOleObjStg record options")
        if rec_instance == 0:
            return record
        return replace(record, body=_transform_storage(record.body))
    if record.children is not None:
        return replace(
            record,
            children=tuple(_transform_tree(child) for child in record.children),
        )
    return record


def _transform_storage(body: bytes) -> bytes:
    if len(body) < 6:
        raise LegacyPptRecordError("truncated ExOleObjStg")
    expected_size = struct.unpack_from("<I", body)[0]
    if expected_size == 0 or expected_size > _MAX_DECOMPRESSED_STORAGE:
        raise LegacyPptRecordError("invalid ExOleObjStg size")
    inflater = zlib.decompressobj()
    storage = inflater.decompress(body[4:], expected_size + 1)
    storage += inflater.flush()
    if (
        len(storage) != expected_size
        or not inflater.eof
        or inflater.unused_data
        or inflater.unconsumed_tail
    ):
        raise LegacyPptRecordError("malformed ExOleObjStg compression")
    cfb = MutableCfb(storage)
    if not cfb.has_root_stream("package_stream"):
        return body
    package = cfb.read_root_stream("package_stream")
    canonical = canonicalize_odf_zip_timestamps(package)
    if canonical == package:
        return body
    cfb.replace_mini_stream("package_stream", canonical)
    rewritten = cfb.bytes()
    return struct.pack("<I", len(rewritten)) + zlib.compress(rewritten, level=9)


def _assign_offsets(
    records: tuple[_Record, ...],
    base: int,
    old_to_new: dict[int, int],
    kinds: dict[int, int],
) -> int:
    position = base
    for record in records:
        if record.old_offset in old_to_new:
            raise LegacyPptRecordError("ambiguous PowerPoint record offset")
        old_to_new[record.old_offset] = position
        kinds[record.old_offset] = record.kind
        if record.children is not None:
            _ = _assign_offsets(record.children, position + 8, old_to_new, kinds)
        position += record.rebuilt_size
    return position


def _serialize(
    record: _Record,
    old_to_new: dict[int, int],
    kinds: dict[int, int],
) -> bytes:
    if record.children is not None:
        body = b"".join(
            _serialize(child, old_to_new, kinds) for child in record.children
        )
    elif record.kind == PERSIST_DIRECTORY_ATOM:
        body = _rewrite_persist_directory(record.body, old_to_new)
    elif record.kind == USER_EDIT_ATOM:
        body = _rewrite_user_edit(record.body, old_to_new, kinds)
    else:
        body = record.body
    return struct.pack("<HHI", record.options, record.kind, len(body)) + body


def _rewrite_persist_directory(body: bytes, old_to_new: dict[int, int]) -> bytes:
    result = bytearray(body)
    offset = 0
    seen_ids: set[int] = set()
    while offset < len(result):
        if offset + 4 > len(result):
            raise LegacyPptRecordError("truncated PersistDirectoryAtom")
        descriptor = struct.unpack_from("<I", result, offset)[0]
        persist_id = descriptor & 0x000FFFFF
        count = descriptor >> 20
        if count == 0 or persist_id == 0 or persist_id + count > 0x00100000:
            raise LegacyPptRecordError("invalid PersistDirectoryAtom entry")
        ids = set(range(persist_id, persist_id + count))
        if seen_ids & ids or offset + 4 + count * 4 > len(result):
            raise LegacyPptRecordError("ambiguous PersistDirectoryAtom")
        seen_ids.update(ids)
        offset += 4
        for _ in range(count):
            old = struct.unpack_from("<I", result, offset)[0]
            if old not in old_to_new:
                raise LegacyPptRecordError("unknown persist object offset")
            struct.pack_into("<I", result, offset, old_to_new[old])
            offset += 4
    return bytes(result)


def _rewrite_user_edit(
    body: bytes,
    old_to_new: dict[int, int],
    kinds: dict[int, int],
) -> bytes:
    if len(body) not in {28, 32}:
        raise LegacyPptRecordError("unsupported UserEditAtom size")
    result = bytearray(body)
    last_edit, persist_directory = struct.unpack_from("<II", body, 8)
    struct.pack_into(
        "<II",
        result,
        8,
        _mapped_offset(last_edit, old_to_new, kinds, USER_EDIT_ATOM, True),
        _mapped_offset(
            persist_directory, old_to_new, kinds, PERSIST_DIRECTORY_ATOM, False
        ),
    )
    return bytes(result)


def _mapped_offset(
    old: int,
    old_to_new: dict[int, int],
    kinds: dict[int, int],
    expected_kind: int,
    zero_is_sentinel: bool,
) -> int:
    if zero_is_sentinel and old == 0:
        return 0
    if old not in old_to_new or kinds[old] != expected_kind:
        raise LegacyPptRecordError("invalid PowerPoint cross-record offset")
    return old_to_new[old]
