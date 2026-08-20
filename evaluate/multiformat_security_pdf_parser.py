from __future__ import annotations

import re
from dataclasses import dataclass
from typing import Final

from evaluate.multiformat_package_validation import MAX_SOURCE_BYTES
from evaluate.multiformat_pdf import MAX_OBJECT_BYTES, MAX_OBJECTS, MAX_XREF_BYTES
from evaluate.multiformat_security_pdf_tokens import PdfObjectId, top_level_name

START_XREF: Final[re.Pattern[bytes]] = re.compile(rb"startxref\s+(\d+)\s+%%EOF")
XREF_ENTRY: Final[re.Pattern[bytes]] = re.compile(rb"^(\d{10})\s+(\d{5})\s+([nf])\s*$")
MAX_FALLBACK_XREF_CANDIDATES: Final[int] = 128


@dataclass(frozen=True, slots=True)
class PdfStructure:
    objects: dict[PdfObjectId, bytes]
    trailer: bytes
    xref_offset: int


def active_pdf_structure(value: bytes) -> PdfStructure | None:
    offset = start_xref_offset(value)
    return parse_pdf_structure(value, offset) if offset is not None else None


def fallback_pdf_structure(value: bytes) -> PdfStructure | None:
    search_end = len(value)
    for _ in range(MAX_FALLBACK_XREF_CANDIDATES):
        marker = value.rfind(b"\nxref\n", 0, search_end)
        offset = marker + 1 if marker >= 0 else 0
        if marker < 0 and not value.startswith(b"xref\n"):
            return None
        structure = parse_pdf_structure(value, offset)
        if structure is not None and catalog_id(structure.objects) is not None:
            return structure
        if offset == 0:
            return None
        search_end = marker
    return None


def start_xref_offset(value: bytes) -> int | None:
    matches = list(START_XREF.finditer(value[-4096:]))
    return int(matches[-1].group(1)) if matches else None


def parse_pdf_structure(value: bytes, offset: int) -> PdfStructure | None:
    if offset >= len(value) or value[offset : offset + 4] != b"xref":
        return None
    parsed = _parse_xref(value, offset)
    if parsed is None:
        return None
    offsets, trailer = parsed
    objects: dict[int, bytes] = {}
    total_bytes = 0
    for object_id, object_offset in offsets.items():
        body = _read_object(value, object_offset, object_id)
        if body is None:
            return None
        total_bytes += len(body)
        if total_bytes > MAX_SOURCE_BYTES:
            return None
        objects[object_id] = body
    return PdfStructure(objects, trailer, offset)


def has_xref_cycle(value: bytes, structure: PdfStructure) -> bool:
    visited: set[int] = set()
    current = structure
    while True:
        if current.xref_offset in visited:
            return True
        visited.add(current.xref_offset)
        previous = named_integer(current.trailer, b"Prev")
        if previous is None:
            return False
        next_structure = parse_pdf_structure(value, previous)
        if next_structure is None:
            return False
        current = next_structure


def catalog_id(objects: dict[PdfObjectId, bytes]) -> PdfObjectId | None:
    return next(
        (
            object_id
            for object_id, body in objects.items()
            if top_level_name(dictionary_view(body), b"Type") == b"Catalog"
        ),
        None,
    )


def dictionary_view(body: bytes) -> bytes:
    stream = re.search(rb"\bstream(?:\r\n|\r|\n)", body)
    value = body[: stream.start()] if stream is not None else body
    value = re.sub(rb"(?m)%[^\r\n]*", b"", value)
    return _remove_literal_strings(value)


def named_integer(value: bytes, name: bytes) -> int | None:
    match = re.search(rb"/" + re.escape(name) + rb"\s+(\d+)\b", value)
    return int(match.group(1)) if match is not None else None


def _parse_xref(
    value: bytes,
    offset: int,
) -> tuple[dict[PdfObjectId, int], bytes] | None:
    section = value[offset : offset + MAX_XREF_BYTES]
    lines = section.splitlines()
    if not lines or lines[0].strip() != b"xref":
        return None
    offsets: dict[PdfObjectId, int] = {}
    object_numbers: set[int] = set()
    index = 1
    while index < len(lines):
        line = lines[index].strip()
        if line == b"trailer":
            trailer = b"\n".join(lines[index + 1 :])
            match = re.search(rb"<<(.*?)>>", trailer, re.DOTALL)
            return (offsets, match.group(1)) if match is not None else None
        header = line.split()
        if len(header) != 2 or not all(item.isdigit() for item in header):
            return None
        first, count = (int(item) for item in header)
        if count < 0 or len(offsets) + count > MAX_OBJECTS:
            return None
        index += 1
        if index + count > len(lines):
            return None
        for item_index in range(count):
            match = XREF_ENTRY.fullmatch(lines[index + item_index])
            if match is None:
                return None
            if match.group(3) == b"n":
                object_number = first + item_index
                object_id = (object_number, int(match.group(2)))
                object_offset = int(match.group(1))
                if object_number in object_numbers or object_offset >= len(value):
                    return None
                offsets[object_id] = object_offset
                object_numbers.add(object_number)
        index += count
    return None


def _read_object(
    value: bytes,
    offset: int,
    object_id: PdfObjectId,
) -> bytes | None:
    candidate = value[offset : offset + MAX_OBJECT_BYTES]
    header = re.match(
        rb"\s*"
        + str(object_id[0]).encode()
        + rb"\s+"
        + str(object_id[1]).encode()
        + rb"\s+obj\b",
        candidate,
    )
    if header is None:
        return None
    end = candidate.find(b"endobj", header.end())
    return candidate[header.end() : end] if end >= 0 else None


def _remove_literal_strings(value: bytes) -> bytes:
    result = bytearray(value)
    depth = 0
    escaped = False
    for index, byte in enumerate(value):
        if depth == 0:
            if byte == ord("("):
                depth = 1
            continue
        if escaped:
            result[index] = ord(" ")
            escaped = False
        elif byte == ord("\\"):
            result[index] = ord(" ")
            escaped = True
        elif byte == ord("("):
            depth += 1
        elif byte == ord(")"):
            depth -= 1
        else:
            result[index] = ord(" ")
    return bytes(result)
