from __future__ import annotations

import re
import zlib
from typing import Final

from evaluate.multiformat_pdf_types import (
    PDF_TYPE1_KEYS,
    PdfConformanceError,
    PdfUnsupportedConstructError,
    pdf_decoded_name,
    pdf_require_name_value,
    pdf_unique_name_values,
)

TYPE1_UNIQUE_ID = re.compile(rb"(/UniqueID\s+)([0-9]+)")
_TYPE1_SEGMENTS: Final = (b"Length1", b"Length2", b"Length3")


def _type1_candidate(dictionary: bytes) -> bool:
    for raw_name in re.findall(rb"/([^\x00\t\n\x0c\r ()<>\[\]{}/%]+)", dictionary):
        try:
            name = pdf_decoded_name(raw_name, "type1-structure")
        except PdfUnsupportedConstructError:
            continue
        if name in _TYPE1_SEGMENTS:
            return True
    return False


def canonicalize_type1_font_objects(
    objects: dict[int, bytes],
) -> dict[int, bytes]:
    result = dict(objects)
    for object_id, value in objects.items():
        stream_start = value.find(b"\nstream\n")
        dictionary = value[:stream_start]
        if stream_start < 0 or not _type1_candidate(dictionary):
            continue
        fields = dict(
            pdf_unique_name_values(dictionary, PDF_TYPE1_KEYS, "type1-structure")
        )
        if not all(name in fields for name in _TYPE1_SEGMENTS):
            continue
        pdf_require_name_value(
            fields.get(b"Filter", b""), b"FlateDecode", "type1-filter"
        )
        length = re.match(rb"\s*([0-9]+)\s+0\s+R", fields.get(b"Length", b""))
        segments = tuple(
            re.match(rb"\s*([0-9]+)", fields[name]) for name in _TYPE1_SEGMENTS
        )
        if length is None or any(segment is None for segment in segments):
            raise PdfUnsupportedConstructError("Type1 lengths", "type1-structure")
        stream_end = value.rfind(b"\nendstream")
        if stream_start < 0 or stream_end < stream_start:
            raise PdfConformanceError("Type1 font stream is invalid")
        stream_start += len(b"\nstream\n")
        try:
            program = zlib.decompress(value[stream_start:stream_end])
        except zlib.error as error:
            raise PdfConformanceError("Type1 font stream is not Flate data") from error
        length1, length2, length3 = (
            int(segment.group(1)) for segment in segments if segment is not None
        )
        if length1 + length2 + length3 != len(program):
            raise PdfConformanceError("Type1 font segment lengths differ")
        encrypted = program[length1 : length1 + length2]
        plaintext = _decrypt(encrypted)
        unique_ids = TYPE1_UNIQUE_ID.findall(plaintext)
        if len(unique_ids) > 1:
            raise PdfUnsupportedConstructError(
                "multiple Type1 UniqueID values are ambiguous", "type1-unique-id"
            )
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
        _update_stream_length(result, int(length.group(1)), len(compressed))
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
    object_id: int,
    length: int,
) -> None:
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
