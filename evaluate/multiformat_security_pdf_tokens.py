from __future__ import annotations

import re
from typing import TypeAlias

NAME_DELIMITERS = b" \t\r\n\f\x00/<>[]()"
PdfObjectId: TypeAlias = tuple[int, int]


def top_level_value(value: bytes, key: bytes) -> bytes | None:
    items = _top_level_items(value)
    return items.get(key) if items is not None else None


def direct_array_references(value: bytes) -> tuple[PdfObjectId, ...] | None:
    items = direct_array_values(value)
    if items is None:
        return None
    return tuple(
        reference for item in items if (reference := reference_value(item)) is not None
    )


def direct_dictionary_references(value: bytes) -> tuple[PdfObjectId, ...] | None:
    items = _top_level_items(value)
    if items is None:
        return None
    return tuple(
        reference
        for item in items.values()
        if (reference := reference_value(item)) is not None
    )


def direct_array_values(value: bytes) -> tuple[bytes, ...] | None:
    stripped = value.strip()
    if not stripped.startswith(b"["):
        return None
    end = _matching_array(stripped, 0, len(stripped))
    if end is None or end != len(stripped):
        return None
    cursor = 1
    limit = end - 1
    result: list[bytes] = []
    while cursor < limit:
        cursor = _skip_space(stripped, cursor, limit)
        if cursor >= limit:
            break
        parsed = _read_value(stripped, cursor, limit)
        if parsed is None:
            return None
        item, cursor = parsed
        result.append(item)
    return tuple(result)


def _top_level_items(value: bytes) -> dict[bytes, bytes] | None:
    bounds = _dictionary_bounds(value)
    if bounds is None:
        return None
    cursor, end = bounds
    result: dict[bytes, bytes] = {}
    while cursor < end:
        cursor = _skip_space(value, cursor, end)
        if cursor >= end:
            break
        if value[cursor] != ord("/"):
            return None
        name_end = _name_end(value, cursor + 1, end)
        name = value[cursor + 1 : name_end]
        value_start = _skip_space(value, name_end, end)
        parsed = _read_value(value, value_start, end)
        if parsed is None:
            return None
        item, cursor = parsed
        if name in result:
            return None
        result[name] = item
    return result


def top_level_reference(value: bytes, key: bytes) -> PdfObjectId | None:
    item = top_level_value(value, key)
    if item is None:
        return None
    match = re.fullmatch(rb"(\d+)\s+(\d+)\s+R", item.strip())
    return (int(match.group(1)), int(match.group(2))) if match is not None else None


def top_level_integer(value: bytes, key: bytes) -> int | None:
    item = top_level_value(value, key)
    if item is None:
        return None
    stripped = item.strip()
    return int(stripped) if stripped.isdigit() else None


def top_level_name(value: bytes, key: bytes) -> bytes | None:
    item = top_level_value(value, key)
    if item is None:
        return None
    stripped = item.strip()
    return stripped[1:] if stripped.startswith(b"/") else None


def reference_value(value: bytes) -> PdfObjectId | None:
    match = re.fullmatch(rb"(\d+)\s+(\d+)\s+R", value.strip())
    return (int(match.group(1)), int(match.group(2))) if match is not None else None


def is_pdf_string(value: bytes) -> bool:
    stripped = value.strip()
    if stripped.startswith(b"("):
        return _matching_literal(stripped, 0, len(stripped)) == len(stripped)
    if (
        not stripped.startswith(b"<")
        or stripped.startswith(b"<<")
        or not stripped.endswith(b">")
    ):
        return False
    return all(
        byte in b"0123456789abcdefABCDEF \t\r\n\f\x00" for byte in stripped[1:-1]
    )


def _dictionary_bounds(value: bytes) -> tuple[int, int] | None:
    start = value.find(b"<<")
    if start < 0:
        return (0, len(value))
    end = _matching_dictionary(value, start, len(value))
    return (start + 2, end - 2) if end is not None else None


def _read_value(
    value: bytes,
    start: int,
    end: int,
) -> tuple[bytes, int] | None:
    if start >= end:
        return None
    if value[start : start + 2] == b"<<":
        item_end = _matching_dictionary(value, start, end)
    elif value[start] == ord("["):
        item_end = _matching_array(value, start, end)
    elif value[start] == ord("("):
        item_end = _matching_literal(value, start, end)
    elif value[start] == ord("/"):
        item_end = _name_end(value, start + 1, end)
    elif value[start] == ord("<"):
        marker = value.find(b">", start + 1, end)
        item_end = marker + 1 if marker >= 0 else None
    else:
        item_end = _scalar_end(value, start, end)
    if item_end is None or item_end <= start:
        return None
    return value[start:item_end], item_end


def _matching_dictionary(value: bytes, start: int, end: int) -> int | None:
    depth = 0
    cursor = start
    while cursor + 1 < end:
        token = value[cursor : cursor + 2]
        if token == b"<<":
            depth += 1
            cursor += 2
            continue
        if token == b">>":
            depth -= 1
            cursor += 2
            if depth == 0:
                return cursor
            continue
        cursor += 1
    return None


def _matching_array(value: bytes, start: int, end: int) -> int | None:
    depth = 0
    cursor = start
    while cursor < end:
        if value[cursor : cursor + 2] == b"<<":
            dictionary_end = _matching_dictionary(value, cursor, end)
            if dictionary_end is None:
                return None
            cursor = dictionary_end
            continue
        if value[cursor] == ord("["):
            depth += 1
        elif value[cursor] == ord("]"):
            depth -= 1
            if depth == 0:
                return cursor + 1
        cursor += 1
    return None


def _matching_literal(value: bytes, start: int, end: int) -> int | None:
    depth = 0
    for cursor in range(start, end):
        if value[cursor] == ord("("):
            depth += 1
        elif value[cursor] == ord(")"):
            depth -= 1
            if depth == 0:
                return cursor + 1
    return None


def _scalar_end(value: bytes, start: int, end: int) -> int:
    first_end = _token_end(value, start, end)
    first = value[start:first_end]
    if not first.isdigit():
        return first_end
    second_start = _skip_space(value, first_end, end)
    second_end = _token_end(value, second_start, end)
    second = value[second_start:second_end]
    reference_start = _skip_space(value, second_end, end)
    reference_end = _token_end(value, reference_start, end)
    if second.isdigit() and value[reference_start:reference_end] == b"R":
        return reference_end
    return first_end


def _skip_space(value: bytes, cursor: int, end: int) -> int:
    while cursor < end and value[cursor] in b" \t\r\n\f\x00":
        cursor += 1
    return cursor


def _name_end(value: bytes, cursor: int, end: int) -> int:
    while cursor < end and value[cursor] not in NAME_DELIMITERS:
        cursor += 1
    return cursor


def _token_end(value: bytes, cursor: int, end: int) -> int:
    while cursor < end and value[cursor] not in NAME_DELIMITERS:
        cursor += 1
    return cursor
