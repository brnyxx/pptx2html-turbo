from __future__ import annotations

import re
import zlib

from evaluate.multiformat_conformance_pdf import PdfConformanceError

TYPE1_LENGTHS = re.compile(
    rb"/Length1\s+([0-9]+)\s+/Length2\s+([0-9]+)\s+/Length3\s+([0-9]+)",
)
TYPE1_UNIQUE_ID = re.compile(rb"(/UniqueID\s+)([0-9]+)")


def canonicalize_type1_font_objects(
    objects: dict[int, bytes],
) -> dict[int, bytes]:
    result = dict(objects)
    for object_id, value in objects.items():
        lengths = TYPE1_LENGTHS.search(value)
        if lengths is None or b"/Filter /FlateDecode" not in value:
            continue
        stream_start = value.find(b"\nstream\n")
        stream_end = value.rfind(b"\nendstream")
        if stream_start < 0 or stream_end < stream_start:
            raise PdfConformanceError("Type1 font stream is invalid")
        stream_start += len(b"\nstream\n")
        try:
            program = zlib.decompress(value[stream_start:stream_end])
        except zlib.error as error:
            raise PdfConformanceError("Type1 font stream is not Flate data") from error
        length1, length2, length3 = (int(lengths.group(index)) for index in range(1, 4))
        if length1 + length2 + length3 != len(program):
            raise PdfConformanceError("Type1 font segment lengths differ")
        encrypted = program[length1 : length1 + length2]
        plaintext = _decrypt(encrypted)
        normalized = TYPE1_UNIQUE_ID.sub(
            lambda match: match.group(1) + b"0" * len(match.group(2)),
            plaintext,
        )
        if normalized == plaintext:
            continue
        canonical_program = (
            program[:length1] + _encrypt(normalized) + program[length1 + length2 :]
        )
        compressed = zlib.compress(canonical_program, level=9)
        result[object_id] = value[:stream_start] + compressed + value[stream_end:]
        _update_stream_length(result, value, len(compressed))
    return result


def _decrypt(value: bytes) -> bytes:
    result = bytearray()
    state = 55665
    for item in value:
        result.append(item ^ (state >> 8))
        state = ((item + state) * 52845 + 22719) & 0xFFFF
    return bytes(result)


def _encrypt(value: bytes) -> bytes:
    result = bytearray()
    state = 55665
    for item in value:
        encrypted = item ^ (state >> 8)
        result.append(encrypted)
        state = ((encrypted + state) * 52845 + 22719) & 0xFFFF
    return bytes(result)


def _update_stream_length(
    objects: dict[int, bytes],
    stream: bytes,
    length: int,
) -> None:
    match = re.search(rb"/Length\s+([0-9]+)\s+0\s+R", stream)
    if match is None:
        raise PdfConformanceError("Type1 font length object is missing")
    object_id = int(match.group(1))
    value = objects.get(object_id)
    if value is None:
        raise PdfConformanceError("Type1 font length object is unavailable")
    updated, count = re.subn(
        rb"([0-9]+\s+0\s+obj\s*)[0-9]+(\s*endobj)",
        lambda item: item.group(1) + str(length).encode() + item.group(2),
        value,
        count=1,
    )
    if count != 1:
        raise PdfConformanceError("Type1 font length object is invalid")
    objects[object_id] = updated
