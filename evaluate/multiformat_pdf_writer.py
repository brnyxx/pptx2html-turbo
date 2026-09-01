from __future__ import annotations

import re
import zlib
from dataclasses import replace
from typing import Final

from evaluate.multiformat_pdf_types import (
    PDF_CATALOG_KEYS,
    PDF_DELIMITERS,
    PDF_METADATA_KEYS,
    ParsedPdfObjects,
    PdfConformanceError,
    PdfUnsupportedConstructError,
    pdf_decoded_name,
    pdf_literal_end,
    pdf_require_name_value,
    pdf_space_comment_end,
    pdf_unique_name_tokens,
    pdf_unique_name_values,
)
from evaluate.multiformat_pdf_xref import parse_pdf_objects

_INFO_DATE_KEYS: Final = (b"CreationDate", b"ModDate")
_XMP_DATE_TAGS: Final = (b"CreateDate", b"ModifyDate", b"MetadataDate")


def _info_date_spans(value: bytes) -> tuple[tuple[int, int], ...]:
    header = re.match(rb"[0-9]+\s+0\s+obj\s*<<", value)
    if header is None:
        raise PdfUnsupportedConstructError(
            "unsupported Info object", "metadata-structure"
        )
    cursor = header.end()
    dictionary_depth = 1
    array_depth = 0
    spans: list[tuple[int, int]] = []
    seen_dates: set[bytes] = set()
    while cursor < len(value) and dictionary_depth:
        token = value[cursor]
        if token in b"\x00\t\n\x0c\r ":
            cursor += 1
        elif token == ord("%"):
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
                raise PdfUnsupportedConstructError(
                    "unterminated Info hex", "metadata-structure"
                )
        elif token == ord("["):
            array_depth += 1
            cursor += 1
        elif token == ord("]"):
            array_depth -= 1
            cursor += 1
        elif token == ord("/"):
            name_end = cursor + 1
            while name_end < len(value) and value[name_end] not in PDF_DELIMITERS:
                name_end += 1
            name = pdf_decoded_name(value[cursor + 1 : name_end], "metadata-structure")
            cursor = name_end
            if dictionary_depth == 1 and array_depth == 0 and name in _INFO_DATE_KEYS:
                if name in seen_dates:
                    raise PdfUnsupportedConstructError(
                        "duplicate Info date", "metadata-structure"
                    )
                seen_dates.add(name)
                scalar_start = pdf_space_comment_end(value, cursor)
                if scalar_start >= len(value) or value[scalar_start] != ord("("):
                    raise PdfUnsupportedConstructError(
                        "unsupported document information value", "metadata-structure"
                    )
                scalar_end = pdf_literal_end(value, scalar_start)
                scalar = value[scalar_start + 1 : scalar_end - 1]
                if b"\\" in scalar or b"(" in scalar or b")" in scalar:
                    raise PdfUnsupportedConstructError(
                        "unsupported document information date", "metadata-structure"
                    )
                spans.append((scalar_start + 1, scalar_end - 1))
                cursor = scalar_end
        else:
            cursor += 1
    if dictionary_depth != 0 or array_depth != 0:
        raise PdfUnsupportedConstructError("unbalanced Info", "metadata-structure")
    token_start = pdf_space_comment_end(value, cursor)
    token_end = token_start
    while token_end < len(value) and value[token_end] not in PDF_DELIMITERS:
        token_end += 1
    trailing_token = value[token_start:token_end]
    if trailing_token == b"stream":
        raise PdfUnsupportedConstructError("Info stream", "metadata-structure")
    if trailing_token != b"endobj":
        raise PdfUnsupportedConstructError("Info trailer", "metadata-structure")
    return tuple(spans)


def rewrite_pdf_xref(value: bytes) -> bytes:
    parsed = parse_pdf_objects(value)
    return write_pdf_objects(parsed, parsed.objects)


def canonicalize_pdf_metadata(
    parsed: ParsedPdfObjects,
    objects: dict[int, bytes],
) -> tuple[ParsedPdfObjects, dict[int, bytes]]:
    result = dict(objects)
    if parsed.info_id is not None:
        source_info = result[parsed.info_id]
        info = bytearray(source_info)
        for start, end in _info_date_spans(source_info):
            info[start:end] = b"0" * (end - start)
        result[parsed.info_id] = bytes(info)

    catalog = result[parsed.root_id]
    catalog_fields = dict(
        pdf_unique_name_values(catalog, PDF_CATALOG_KEYS, "catalog-structure")
    )
    metadata_ref = re.match(
        rb"\s*([0-9]+)\s+0\s+R", catalog_fields.get(b"Metadata", b"")
    )
    if b"Metadata" in catalog_fields and metadata_ref is None:
        raise PdfUnsupportedConstructError(
            "unsupported catalog metadata reference", "metadata-structure"
        )
    if metadata_ref:
        metadata_id = int(metadata_ref.group(1))
        metadata = result.get(metadata_id)
        if metadata is None:
            raise PdfConformanceError("PDF catalog Metadata reference is unresolved")
        stream = re.match(
            rb"[0-9]+\s+0\s+obj\s*(<<.*?>>)\s*stream\r?\n",
            metadata,
            flags=re.DOTALL,
        )
        if stream is None:
            raise PdfUnsupportedConstructError(
                "unsupported XMP metadata structure", "metadata-structure"
            )
        dictionary = stream.group(1)
        metadata_tokens = {
            token.name: token
            for token in pdf_unique_name_tokens(
                dictionary, PDF_METADATA_KEYS, "metadata-structure"
            )
        }
        metadata_fields = {
            name: dictionary[token.value_start :]
            for name, token in metadata_tokens.items()
        }
        pdf_require_name_value(
            metadata_fields.get(b"Type", b""), b"Metadata", "metadata-structure"
        )
        pdf_require_name_value(
            metadata_fields.get(b"Subtype", b""), b"XML", "metadata-structure"
        )
        if b"DecodeParms" in metadata_fields:
            raise PdfUnsupportedConstructError("XMP DecodeParms", "metadata-filter")
        filtered = b"Filter" in metadata_fields
        if filtered:
            pdf_require_name_value(
                metadata_fields[b"Filter"], b"FlateDecode", "metadata-filter"
            )
        length_token = metadata_tokens.get(b"Length")
        length_match = re.match(
            rb"\s*([0-9]+)(?!\s+0\s+R)", metadata_fields.get(b"Length", b"")
        )
        if length_token is None or length_match is None:
            raise PdfUnsupportedConstructError("XMP Length", "metadata-structure")
        stream_end = metadata.rfind(b"\nendstream")
        if stream_end < stream.end():
            raise PdfConformanceError("PDF XMP metadata stream is invalid")
        encoded = metadata[stream.end() : stream_end]
        if int(length_match.group(1)) != len(encoded):
            raise PdfConformanceError("PDF XMP metadata length differs")
        try:
            payload = zlib.decompress(encoded) if filtered else encoded
        except zlib.error as error:
            raise PdfConformanceError("PDF XMP metadata is not Flate data") from error
        for tag in _XMP_DATE_TAGS:
            pattern = re.compile(rb"(<xmp:" + tag + rb">)([^<]*)(</xmp:" + tag + rb">)")
            payload = pattern.sub(
                lambda match: (
                    match.group(1) + b"0" * len(match.group(2)) + match.group(3)
                ),
                payload,
            )
        encoded = zlib.compress(payload, level=9) if filtered else payload
        length_start = length_token.value_start + length_match.start(1)
        length_end = length_token.value_start + length_match.end(1)
        dictionary = (
            dictionary[:length_start]
            + str(len(encoded)).encode()
            + dictionary[length_end:]
        )
        result[metadata_id] = (
            metadata[: stream.start(1)]
            + dictionary
            + metadata[stream.end(1) : stream.end()]
            + encoded
            + metadata[stream_end:]
        )

    document_id = parsed.document_id
    if document_id is not None:
        document_id = re.sub(
            rb"(<)([0-9A-Fa-f]+)(>)",
            lambda match: match.group(1) + b"0" * len(match.group(2)) + match.group(3),
            document_id,
        )
    checksum = parsed.document_checksum
    if checksum is not None:
        checksum = re.sub(
            rb"(/DocChecksum\s*/)([0-9A-Fa-f]+)",
            lambda match: match.group(1) + b"0" * len(match.group(2)),
            checksum,
        )
    return replace(parsed, document_id=document_id, document_checksum=checksum), result


def write_pdf_objects(
    parsed: ParsedPdfObjects,
    objects: dict[int, bytes],
) -> bytes:
    prefix = parsed.prefix
    if not prefix.endswith(b"\n"):
        prefix += b"\n"
    result = bytearray(prefix)
    offsets: dict[int, int] = {}
    for object_id in sorted(objects):
        offsets[object_id] = len(result)
        result.extend(objects[object_id].rstrip() + b"\n")
    xref_offset = len(result)
    size = max(objects) + 1
    result.extend(f"xref\n0 {size}\n".encode())
    result.extend(b"0000000000 65535 f \n")
    for object_id in range(1, size):
        offset = offsets.get(object_id)
        if offset is None:
            result.extend(b"0000000000 00000 f \n")
        else:
            result.extend(f"{offset:010d} 00000 n \n".encode())
    info = f" /Info {parsed.info_id} 0 R" if parsed.info_id is not None else ""
    document_id = b" " + parsed.document_id if parsed.document_id is not None else b""
    checksum = (
        b" " + parsed.document_checksum if parsed.document_checksum is not None else b""
    )
    result.extend(
        f"trailer\n<< /Size {size} /Root {parsed.root_id} 0 R{info}".encode()
        + document_id
        + checksum
        + f" >>\nstartxref\n{xref_offset}\n%%EOF\n".encode()
    )
    return bytes(result)
