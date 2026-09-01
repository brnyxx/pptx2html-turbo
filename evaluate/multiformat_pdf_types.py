"""Shared PDF structure types, constants, and dictionary parsing.

This module is the dependency leaf of the canonicalizer: it imports nothing
from the rest of the cluster. The xref parser, the writer, the Type1
normalizer, the object renumberer, and the canonicalizer entry point all depend
on it, so the import graph stays acyclic and every stage agrees on one
definition of a parsed document and one PDF name grammar.
"""

from __future__ import annotations

import re
from collections.abc import Collection
from dataclasses import dataclass
from typing import Final

PDF_DELIMITERS: Final = b"\x00\t\n\x0c\r ()<>[]{}/%"
PDF_TRAILER_KEYS: Final = frozenset(
    {b"Root", b"Info", b"Encrypt", b"Prev", b"XRefStm", b"ID", b"DocChecksum", b"Size"}
)
PDF_XREF_STREAM_KEYS: Final = PDF_TRAILER_KEYS | frozenset(
    {b"Type", b"W", b"Index", b"Length", b"Filter", b"DecodeParms"}
)
PDF_OBJECT_STREAM_KEYS: Final = frozenset(
    {b"Type", b"Length", b"Filter", b"DecodeParms", b"N", b"First"}
)
PDF_CATALOG_KEYS: Final = frozenset({b"Type", b"Metadata"})
PDF_METADATA_KEYS: Final = frozenset(
    {b"Type", b"Subtype", b"Length", b"Filter", b"DecodeParms"}
)
PDF_TYPE1_KEYS: Final = frozenset(
    {b"Length", b"Filter", b"Length1", b"Length2", b"Length3"}
)
_UNSUPPORTED_FIELDS: Final = (
    (b"Encrypt", "encryption"),
    (b"Prev", "incremental-update"),
    (b"XRefStm", "hybrid-xref"),
)


@dataclass(frozen=True, slots=True)
class PdfConformanceError(Exception):
    reason: str

    def __str__(self) -> str:
        return self.reason


@dataclass(frozen=True, slots=True)
class PdfUnsupportedConstructError(PdfConformanceError):
    construct: str


@dataclass(frozen=True, slots=True)
class PdfNameToken:
    name: bytes
    value_start: int


@dataclass(frozen=True, slots=True)
class ParsedPdfObjects:
    prefix: bytes
    objects: dict[int, bytes]
    root_id: int
    info_id: int | None
    document_id: bytes | None
    document_checksum: bytes | None


def pdf_space_comment_end(value: bytes, start: int) -> int:
    cursor = start
    while cursor < len(value):
        if value[cursor] in b"\x00\t\n\x0c\r ":
            cursor += 1
        elif value[cursor] == ord("%"):
            ending = re.search(rb"\r\n?|\n", value[cursor:])
            cursor = len(value) if ending is None else cursor + ending.end()
        else:
            return cursor
    return cursor


def pdf_decoded_name(value: bytes, construct: str) -> bytes:
    result = bytearray()
    cursor = 0
    while cursor < len(value):
        if value[cursor] != ord("#"):
            result.append(value[cursor])
            cursor += 1
            continue
        escape = value[cursor + 1 : cursor + 3]
        if len(escape) != 2 or re.fullmatch(rb"[0-9A-Fa-f]{2}", escape) is None:
            raise PdfUnsupportedConstructError("malformed PDF name escape", construct)
        result.append(int(escape, 16))
        cursor += 3
    return bytes(result)


def pdf_literal_end(value: bytes, start: int) -> int:
    cursor = start + 1
    depth = 1
    while cursor < len(value):
        token = value[cursor]
        if token == ord("\\"):
            cursor += 2
            continue
        if token == ord("("):
            depth += 1
        elif token == ord(")"):
            depth -= 1
            if depth == 0:
                return cursor + 1
        cursor += 1
    raise PdfUnsupportedConstructError("unterminated Info string", "metadata-structure")


def pdf_top_level_names(value: bytes, construct: str) -> tuple[PdfNameToken, ...]:
    dictionary_start = value.find(b"<<")
    if dictionary_start < 0:
        raise PdfUnsupportedConstructError("PDF dictionary is missing", construct)
    cursor = dictionary_start + 2
    dictionary_depth = 1
    array_depth = 0
    tokens: list[PdfNameToken] = []
    while cursor < len(value) and dictionary_depth:
        token = value[cursor]
        if token == ord("%"):
            cursor = pdf_space_comment_end(value, cursor)
        elif token == ord("("):
            cursor = pdf_literal_end(value, cursor)
        elif value[cursor : cursor + 2] == b"<<":
            dictionary_depth += 1
            cursor += 2
        elif value[cursor : cursor + 2] == b">>":
            dictionary_depth -= 1
            cursor += 2
        elif token == ord("<"):
            cursor = value.find(b">", cursor + 1) + 1
            if cursor == 0:
                raise PdfUnsupportedConstructError("unterminated PDF hex", construct)
        elif token == ord("["):
            array_depth += 1
            cursor += 1
        elif token == ord("]"):
            array_depth -= 1
            cursor += 1
        elif token == ord("/") and dictionary_depth == 1 and array_depth == 0:
            name_end = cursor + 1
            while name_end < len(value) and value[name_end] not in PDF_DELIMITERS:
                name_end += 1
            name = pdf_decoded_name(value[cursor + 1 : name_end], construct)
            tokens.append(PdfNameToken(name, name_end))
            cursor = name_end
        else:
            cursor += 1
    if dictionary_depth != 0 or array_depth != 0:
        raise PdfUnsupportedConstructError("unbalanced PDF dictionary", construct)
    return tuple(tokens)


def pdf_unique_name_tokens(
    value: bytes,
    supported: frozenset[bytes],
    construct: str,
) -> tuple[PdfNameToken, ...]:
    result: list[PdfNameToken] = []
    seen: set[bytes] = set()
    for token in pdf_top_level_names(value, construct):
        if token.name not in supported:
            continue
        if token.name in seen:
            raise PdfUnsupportedConstructError("duplicate PDF field", construct)
        seen.add(token.name)
        result.append(token)
    return tuple(result)


def pdf_unique_name_values(
    value: bytes,
    supported: frozenset[bytes],
    construct: str,
) -> tuple[tuple[bytes, bytes], ...]:
    return tuple(
        (token.name, value[token.value_start :])
        for token in pdf_unique_name_tokens(value, supported, construct)
    )


def pdf_name_value(value: bytes, construct: str) -> bytes:
    start = pdf_space_comment_end(value, 0)
    if start >= len(value) or value[start] != ord("/"):
        raise PdfUnsupportedConstructError("PDF name value is missing", construct)
    end = start + 1
    while end < len(value) and value[end] not in PDF_DELIMITERS:
        end += 1
    return pdf_decoded_name(value[start + 1 : end], construct)


def pdf_reject_unsupported_fields(names: Collection[bytes]) -> None:
    for name, construct in _UNSUPPORTED_FIELDS:
        if name in names:
            raise PdfUnsupportedConstructError(f"unsupported {construct}", construct)


def pdf_require_name_value(value: bytes, expected: bytes, construct: str) -> None:
    if pdf_name_value(value, construct) != expected:
        raise PdfUnsupportedConstructError("unsupported PDF name value", construct)


def pdf_trailer_identity(
    value: bytes,
) -> tuple[int, int | None, bytes | None, bytes | None]:
    fields = {
        token.name: value[token.value_start :]
        for token in pdf_unique_name_tokens(
            value, PDF_TRAILER_KEYS, "trailer-structure"
        )
    }
    pdf_reject_unsupported_fields(fields)
    root = re.match(rb"\s*([0-9]+)\s+0\s+R", fields.get(b"Root", b""))
    size = re.match(rb"\s*([0-9]+)", fields.get(b"Size", b""))
    if root is None or size is None:
        raise PdfConformanceError("PDF trailer Root or Size is missing")
    info = re.match(rb"\s*([0-9]+)\s+0\s+R", fields.get(b"Info", b""))
    document_id = re.match(rb"\s*(\[\s*<[^>]+>\s*<[^>]+>\s*\])", fields.get(b"ID", b""))
    checksum = re.match(rb"\s*/([0-9A-Fa-f]+)", fields.get(b"DocChecksum", b""))
    id_value = b"/ID " + document_id.group(1) if document_id else None
    checksum_value = b"/DocChecksum /" + checksum.group(1) if checksum else None
    return (
        int(root.group(1)),
        int(info.group(1)) if info else None,
        id_value,
        checksum_value,
    )
