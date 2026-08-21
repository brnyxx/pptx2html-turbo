from __future__ import annotations

import re
import zlib
from dataclasses import dataclass

from evaluate.multiformat_conformance_pdf import PdfConformanceError


@dataclass(frozen=True, slots=True)
class ParsedPdfObjects:
    prefix: bytes
    objects: dict[int, bytes]
    root_id: int


def parse_pdf_objects(value: bytes) -> ParsedPdfObjects:
    xref_offset = _startxref(value)
    offsets, compressed, root_id = _xref_metadata(value, xref_offset)
    objects = _objects(value, offsets, compressed, xref_offset)
    prefix = value[: min(offsets.values())]
    return ParsedPdfObjects(prefix, objects, root_id)


def _startxref(value: bytes) -> int:
    matches = list(re.finditer(rb"startxref\s+([0-9]+)\s+%%EOF", value))
    if not matches:
        raise PdfConformanceError("PDF startxref is unavailable")
    offset = int(matches[-1].group(1))
    if offset < 0 or offset >= len(value):
        raise PdfConformanceError("PDF startxref is outside the file")
    return offset


def _xref_metadata(
    value: bytes,
    xref_offset: int,
) -> tuple[dict[int, int], dict[int, tuple[int, int]], int]:
    if value[xref_offset : xref_offset + 4] == b"xref":
        return (
            _xref_table_offsets(value, xref_offset),
            {},
            _required_int(rb"/Root\s+([0-9]+)\s+0\s+R", value[xref_offset:]),
        )
    return _xref_stream_offsets(value, xref_offset)


def _xref_table_offsets(value: bytes, xref_offset: int) -> dict[int, int]:
    trailer_offset = value.find(b"trailer", xref_offset)
    if trailer_offset < 0:
        raise PdfConformanceError("PDF trailer is unavailable")
    lines = value[xref_offset:trailer_offset].splitlines()
    if not lines or lines[0].strip() != b"xref":
        raise PdfConformanceError("PDF xref header is invalid")
    offsets: dict[int, int] = {}
    index = 1
    while index < len(lines):
        header = lines[index].split()
        index += 1
        if len(header) != 2:
            raise PdfConformanceError("PDF xref subsection is invalid")
        start = int(header[0])
        count = int(header[1])
        if index + count > len(lines):
            raise PdfConformanceError("PDF xref entries are incomplete")
        for entry_index in range(count):
            entry = lines[index + entry_index].split()
            if len(entry) != 3:
                raise PdfConformanceError("PDF xref entry is invalid")
            if entry[2] == b"n":
                object_id = start + entry_index
                if object_id in offsets or entry[1] != b"00000":
                    raise PdfConformanceError("PDF xref generation is unsupported")
                offsets[object_id] = int(entry[0])
        index += count
    if not offsets:
        raise PdfConformanceError("PDF xref has no objects")
    return offsets


def _xref_stream_offsets(
    value: bytes,
    xref_offset: int,
) -> tuple[dict[int, int], dict[int, tuple[int, int]], int]:
    object_match = re.match(
        rb"([0-9]+)\s+0\s+obj\s*(<<.*?>>)\s*stream\r?\n",
        value[xref_offset:],
        flags=re.DOTALL,
    )
    if object_match is None:
        raise PdfConformanceError("PDF xref stream object is invalid")
    dictionary = object_match.group(2)
    widths = _required_ints(rb"/W\s*\[\s*([0-9 ]+)\]", dictionary)
    if len(widths) != 3 or sum(widths) <= 0:
        raise PdfConformanceError("PDF xref stream widths are invalid")
    size = _required_int(rb"/Size\s+([0-9]+)", dictionary)
    indexes_match = re.search(rb"/Index\s*\[\s*([0-9 ]+)\]", dictionary)
    indexes = (
        _parse_ints(indexes_match.group(1)) if indexes_match is not None else [0, size]
    )
    if len(indexes) % 2 != 0:
        raise PdfConformanceError("PDF xref stream index is invalid")
    length = _required_int(rb"/Length\s+([0-9]+)", dictionary)
    stream_start = xref_offset + object_match.end()
    try:
        decoded = zlib.decompress(value[stream_start : stream_start + length])
    except zlib.error as error:
        raise PdfConformanceError("PDF xref stream is not Flate data") from error
    entry_width = sum(widths)
    expected_entries = sum(indexes[index + 1] for index in range(0, len(indexes), 2))
    if len(decoded) != expected_entries * entry_width:
        raise PdfConformanceError("PDF xref stream length differs")
    offsets: dict[int, int] = {}
    compressed: dict[int, tuple[int, int]] = {}
    cursor = 0
    for index in range(0, len(indexes), 2):
        start = indexes[index]
        count = indexes[index + 1]
        for object_id in range(start, start + count):
            fields = []
            for width in widths:
                fields.append(
                    int.from_bytes(decoded[cursor : cursor + width], "big")
                    if width
                    else 0
                )
                cursor += width
            entry_type = fields[0] if widths[0] else 1
            if entry_type == 1 and fields[1] < xref_offset:
                if object_id in offsets or fields[2] != 0:
                    raise PdfConformanceError(
                        "PDF xref stream generation is unsupported"
                    )
                offsets[object_id] = fields[1]
            elif entry_type == 2:
                compressed[object_id] = (fields[1], fields[2])
    if not offsets:
        raise PdfConformanceError("PDF xref stream has no objects")
    return (
        offsets,
        compressed,
        _required_int(rb"/Root\s+([0-9]+)\s+0\s+R", dictionary),
    )


def _objects(
    value: bytes,
    offsets: dict[int, int],
    compressed: dict[int, tuple[int, int]],
    xref_offset: int,
) -> dict[int, bytes]:
    ordered = sorted(offsets.items(), key=lambda item: item[1])
    result: dict[int, bytes] = {}
    for index, (object_id, offset) in enumerate(ordered):
        end = ordered[index + 1][1] if index + 1 < len(ordered) else xref_offset
        item = value[offset:end].strip()
        if not item.startswith(f"{object_id} 0 obj".encode()) or not item.endswith(
            b"endobj"
        ):
            raise PdfConformanceError("PDF indirect object is invalid")
        result[object_id] = item
    _expand_object_streams(result, compressed)
    return result


def _expand_object_streams(
    objects: dict[int, bytes],
    compressed: dict[int, tuple[int, int]],
) -> None:
    stream_ids = {stream_id for stream_id, _ in compressed.values()}
    streams = {
        stream_id: _object_stream_members(objects[stream_id])
        for stream_id in stream_ids
        if stream_id in objects
    }
    if len(streams) != len(stream_ids):
        raise PdfConformanceError("PDF object stream is missing")
    length_ids = {
        _required_int(rb"/Length\s+([0-9]+)\s+0\s+R", objects[stream_id])
        for stream_id in stream_ids
    }
    for object_id, (stream_id, index) in compressed.items():
        members = streams[stream_id]
        if index >= len(members) or members[index][0] != object_id:
            raise PdfConformanceError("PDF object stream index differs")
        body = members[index][1]
        objects[object_id] = (
            f"{object_id} 0 obj\n".encode() + body.strip() + b"\nendobj"
        )
    for stream_id in stream_ids:
        objects.pop(stream_id)
    for length_id in length_ids:
        reference = f"{length_id} 0 R".encode()
        if not any(reference in value for value in objects.values()):
            objects.pop(length_id, None)


def _object_stream_members(value: bytes) -> list[tuple[int, bytes]]:
    match = re.match(
        rb"[0-9]+\s+0\s+obj\s*(<<.*?>>)\s*stream\r?\n",
        value,
        flags=re.DOTALL,
    )
    if match is None:
        raise PdfConformanceError("PDF object stream syntax is invalid")
    dictionary = match.group(1)
    count = _required_int(rb"/N\s+([0-9]+)", dictionary)
    first = _required_int(rb"/First\s+([0-9]+)", dictionary)
    stream_end = value.rfind(b"\nendstream")
    if stream_end < match.end():
        raise PdfConformanceError("PDF object stream data is missing")
    try:
        decoded = zlib.decompress(value[match.end() : stream_end])
    except zlib.error as error:
        raise PdfConformanceError("PDF object stream is not Flate data") from error
    header = _parse_ints(decoded[:first])
    if len(header) != count * 2:
        raise PdfConformanceError("PDF object stream header differs")
    result = []
    for index in range(count):
        object_id = header[index * 2]
        start = first + header[index * 2 + 1]
        end = first + header[index * 2 + 3] if index + 1 < count else len(decoded)
        if not first <= start <= end <= len(decoded):
            raise PdfConformanceError("PDF object stream offset is invalid")
        result.append((object_id, decoded[start:end]))
    return result


def _required_int(pattern: bytes, value: bytes) -> int:
    match = re.search(pattern, value)
    if match is None:
        raise PdfConformanceError("PDF trailer reference is missing")
    return int(match.group(1))


def _required_ints(pattern: bytes, value: bytes) -> list[int]:
    match = re.search(pattern, value)
    if match is None:
        raise PdfConformanceError("PDF integer array is missing")
    return _parse_ints(match.group(1))


def _parse_ints(value: bytes) -> list[int]:
    try:
        return [int(item) for item in value.split()]
    except ValueError as error:
        raise PdfConformanceError("PDF integer array is invalid") from error
