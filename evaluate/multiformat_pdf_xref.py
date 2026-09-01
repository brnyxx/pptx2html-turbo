from __future__ import annotations

import re
import zlib

from evaluate.multiformat_pdf_types import (
    PDF_CATALOG_KEYS,
    PDF_OBJECT_STREAM_KEYS,
    PDF_XREF_STREAM_KEYS,
    ParsedPdfObjects,
    PdfConformanceError,
    PdfUnsupportedConstructError,
    pdf_reject_unsupported_fields,
    pdf_require_name_value,
    pdf_trailer_identity,
    pdf_unique_name_values,
)


def parse_pdf_objects(value: bytes) -> ParsedPdfObjects:
    if re.match(rb"%PDF-(?:1\.[4-7]|2\.0)(?:\r?\n)", value) is None:
        raise PdfUnsupportedConstructError("PDF version is unsupported", "version")
    xref_offset = _startxref(value)
    offsets, compressed, dictionary = _xref_metadata(value, xref_offset)
    root_id, info_id, document_id, checksum = pdf_trailer_identity(dictionary)
    objects = _objects(value, offsets, compressed, xref_offset)
    if info_id is not None and info_id not in objects:
        raise PdfConformanceError("PDF trailer Info reference is unresolved")
    catalog = objects.get(root_id, b"")
    catalog_fields = dict(
        pdf_unique_name_values(catalog, PDF_CATALOG_KEYS, "catalog-structure")
    )
    pdf_require_name_value(
        catalog_fields.get(b"Type", b""), b"Catalog", "catalog-structure"
    )
    prefix = value[: min(offsets.values())]
    return ParsedPdfObjects(prefix, objects, root_id, info_id, document_id, checksum)


def _startxref(value: bytes) -> int:
    matches = list(re.finditer(rb"startxref\s+([0-9]+)\s+%%EOF", value))
    if not matches:
        raise PdfConformanceError("PDF startxref is unavailable")
    offset = int(matches[-1].group(1))
    if offset >= len(value):
        raise PdfConformanceError("PDF startxref is outside the file")
    return offset


def _xref_metadata(
    value: bytes,
    xref_offset: int,
) -> tuple[dict[int, int], dict[int, tuple[int, int]], bytes]:
    if value[xref_offset : xref_offset + 4] == b"xref":
        trailer = value.find(b"trailer", xref_offset)
        startxref = value.find(b"startxref", trailer)
        if trailer < 0 or startxref < 0:
            raise PdfConformanceError("PDF trailer is unavailable")
        return _xref_table_offsets(value, xref_offset), {}, value[trailer:startxref]
    return _xref_stream_offsets(value, xref_offset)


def _xref_table_offsets(value: bytes, xref_offset: int) -> dict[int, int]:
    trailer_offset = value.find(b"trailer", xref_offset)
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
) -> tuple[dict[int, int], dict[int, tuple[int, int]], bytes]:
    object_match = re.match(
        rb"([0-9]+)\s+0\s+obj\s*(<<.*?>>)\s*stream\r?\n",
        value[xref_offset:],
        flags=re.DOTALL,
    )
    if object_match is None:
        raise PdfConformanceError("PDF xref stream object is invalid")
    dictionary = object_match.group(2)
    fields = dict(
        pdf_unique_name_values(dictionary, PDF_XREF_STREAM_KEYS, "xref-structure")
    )
    pdf_reject_unsupported_fields(fields)
    if b"DecodeParms" in fields:
        raise PdfUnsupportedConstructError(
            "unsupported decode parameters", "xref-filter"
        )
    pdf_require_name_value(fields.get(b"Type", b""), b"XRef", "xref-structure")
    pdf_require_name_value(fields.get(b"Filter", b""), b"FlateDecode", "xref-filter")
    widths_match = re.match(rb"\s*\[\s*([0-9 ]+)\]", fields.get(b"W", b""))
    size_match = re.match(rb"\s*([0-9]+)", fields.get(b"Size", b""))
    length_match = re.match(rb"\s*([0-9]+)", fields.get(b"Length", b""))
    if widths_match is None or size_match is None or length_match is None:
        raise PdfConformanceError("PDF xref stream fields are missing")
    widths = _parse_ints(widths_match.group(1))
    if len(widths) != 3 or sum(widths) <= 0:
        raise PdfConformanceError("PDF xref stream widths are invalid")
    size = int(size_match.group(1))
    indexes_match = re.match(rb"\s*\[\s*([0-9 ]+)\]", fields.get(b"Index", b""))
    indexes = _parse_ints(indexes_match.group(1)) if indexes_match else [0, size]
    if len(indexes) % 2 != 0:
        raise PdfConformanceError("PDF xref stream index is invalid")
    length = int(length_match.group(1))
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
    return offsets, compressed, dictionary


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
    length_ids = {members[1] for members in streams.values()}
    for object_id, (stream_id, index) in compressed.items():
        members = streams[stream_id][0]
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


def _object_stream_members(value: bytes) -> tuple[list[tuple[int, bytes]], int]:
    match = re.match(
        rb"[0-9]+\s+0\s+obj\s*(<<.*?>>)\s*stream\r?\n",
        value,
        flags=re.DOTALL,
    )
    if match is None:
        raise PdfConformanceError("PDF object stream syntax is invalid")
    dictionary = match.group(1)
    fields = dict(
        pdf_unique_name_values(dictionary, PDF_OBJECT_STREAM_KEYS, "object-structure")
    )
    if b"DecodeParms" in fields:
        raise PdfUnsupportedConstructError("object DecodeParms", "object-filter")
    pdf_require_name_value(fields.get(b"Type", b""), b"ObjStm", "object-structure")
    pdf_require_name_value(fields.get(b"Filter", b""), b"FlateDecode", "object-filter")
    length_match = re.match(rb"\s*([0-9]+)\s+0\s+R", fields.get(b"Length", b""))
    count_match = re.match(rb"\s*([0-9]+)", fields.get(b"N", b""))
    first_match = re.match(rb"\s*([0-9]+)", fields.get(b"First", b""))
    if length_match is None or count_match is None or first_match is None:
        raise PdfUnsupportedConstructError("object stream fields", "object-structure")
    length_id = int(length_match.group(1))
    count = int(count_match.group(1))
    first = int(first_match.group(1))
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
    return result, length_id


def _parse_ints(value: bytes) -> list[int]:
    try:
        return [int(item) for item in value.split()]
    except ValueError as error:
        raise PdfConformanceError("PDF integer array is invalid") from error
