from __future__ import annotations

import mmap
import re
from pathlib import Path
from typing import Final

from evaluate.multiformat_package_validation import MAX_SOURCE_BYTES

PDF_HEADER: Final[re.Pattern[bytes]] = re.compile(rb"%PDF-(?:1\.[0-7]|2\.0)")
START_XREF: Final[re.Pattern[bytes]] = re.compile(
    rb"startxref\s+(\d+)\s+%%EOF",
)
XREF_ENTRY: Final[re.Pattern[bytes]] = re.compile(
    rb"^(\d{10})\s+(\d{5})\s+([nf])\s*$",
)
REFERENCE: Final[re.Pattern[bytes]] = re.compile(rb"(\d+)\s+(\d+)\s+R")
MAX_XREF_BYTES: Final[int] = 16 * 1024 * 1024
MAX_OBJECT_BYTES: Final[int] = 1024 * 1024
MAX_OBJECTS: Final[int] = 100_000
MAX_PAGE_DEPTH: Final[int] = 64
MAX_PAGES: Final[int] = 10_000


def valid_pdf(path: Path) -> bool:
    try:
        size = path.stat().st_size
        if not 0 < size <= MAX_SOURCE_BYTES:
            return False
        with path.open("rb") as source:
            with mmap.mmap(source.fileno(), 0, access=mmap.ACCESS_READ) as data:
                return _valid_pdf_mapping(data, size)
    except (OSError, ValueError):
        return False


def valid_pdf_bytes(value: bytes) -> bool:
    size = len(value)
    return 0 < size <= MAX_SOURCE_BYTES and _valid_pdf_mapping(value, size)


def _valid_pdf_mapping(data: bytes | mmap.mmap, size: int) -> bool:
    if PDF_HEADER.match(data[:8]) is None:
        return False
    trailer_window = data[max(0, size - 4096) : size]
    matches = list(START_XREF.finditer(trailer_window))
    if not matches:
        return False
    xref_offset = int(matches[-1].group(1))
    if xref_offset >= size or data[xref_offset : xref_offset + 4] != b"xref":
        return False
    parsed = _parse_xref(data, xref_offset)
    if parsed is None:
        return False
    offsets, trailer = parsed
    if b"/Encrypt" in trailer:
        return False
    root_reference = _named_reference(trailer, b"Root")
    if root_reference is None:
        return False
    root = _read_object(data, offsets, root_reference)
    if root is None or re.search(rb"/Type\s*/Catalog\b", root) is None:
        return False
    pages_reference = _named_reference(root, b"Pages")
    if pages_reference is None:
        return False
    page_count = _page_count(data, offsets, pages_reference, set(), 0)
    return page_count is not None and 0 < page_count <= MAX_PAGES


def _parse_xref(
    data: bytes | mmap.mmap,
    offset: int,
) -> tuple[dict[int, int], bytes] | None:
    section = bytes(data[offset : offset + MAX_XREF_BYTES])
    lines = section.splitlines()
    if not lines or lines[0].strip() != b"xref":
        return None
    offsets: dict[int, int] = {}
    index = 1
    while index < len(lines):
        line = lines[index].strip()
        if line == b"trailer":
            trailer = b"\n".join(lines[index + 1 :])
            match = re.search(rb"<<(.*?)>>", trailer, re.DOTALL)
            return (offsets, match.group(1)) if match is not None else None
        header = line.split()
        if len(header) != 2 or not all(value.isdigit() for value in header):
            return None
        first, count = (int(value) for value in header)
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
                object_id = first + item_index
                object_offset = int(match.group(1))
                if object_id in offsets or object_offset >= len(data):
                    return None
                offsets[object_id] = object_offset
        index += count
    return None


def _read_object(
    data: bytes | mmap.mmap,
    offsets: dict[int, int],
    reference: int,
) -> bytes | None:
    offset = offsets.get(reference)
    if offset is None:
        return None
    value = bytes(data[offset : offset + MAX_OBJECT_BYTES])
    header = re.match(rb"\s*" + str(reference).encode() + rb"\s+\d+\s+obj\b", value)
    if header is None:
        return None
    end = value.find(b"endobj", header.end())
    return value[header.end() : end] if end >= 0 else None


def _named_reference(value: bytes, name: bytes) -> int | None:
    match = re.search(rb"/" + re.escape(name) + rb"\s+" + REFERENCE.pattern, value)
    return int(match.group(1)) if match is not None else None


def _page_count(
    data: bytes | mmap.mmap,
    offsets: dict[int, int],
    reference: int,
    visited: set[int],
    depth: int,
) -> int | None:
    if depth > MAX_PAGE_DEPTH or reference in visited:
        return None
    visited.add(reference)
    value = _read_object(data, offsets, reference)
    if value is None:
        return None
    if re.search(rb"/Type\s*/Page\b", value) is not None:
        return 1
    if re.search(rb"/Type\s*/Pages\b", value) is None:
        return None
    kids_match = re.search(rb"/Kids\s*\[(.*?)\]", value, re.DOTALL)
    count_match = re.search(rb"/Count\s+(\d+)\b", value)
    if kids_match is None or count_match is None:
        return None
    children = [
        int(match.group(1)) for match in REFERENCE.finditer(kids_match.group(1))
    ]
    if not children:
        return None
    total = 0
    for child in children:
        child_count = _page_count(data, offsets, child, visited, depth + 1)
        if child_count is None:
            return None
        total += child_count
        if total > MAX_PAGES:
            return None
    return total if total == int(count_match.group(1)) else None
