from __future__ import annotations

from evaluate.multiformat_conformance_pdf import normalize_pdf_bytes
from evaluate.multiformat_pdf_type1 import canonicalize_type1_font_objects
from evaluate.multiformat_pdf_xref import ParsedPdfObjects, parse_pdf_objects


def canonicalize_pdf_bytes(value: bytes) -> bytes:
    parsed = parse_pdf_objects(value)
    objects = canonicalize_type1_font_objects(parsed.objects)
    return normalize_pdf_bytes(write_pdf_objects(parsed, objects))


def rewrite_pdf_xref(value: bytes) -> bytes:
    parsed = parse_pdf_objects(value)
    return write_pdf_objects(parsed, parsed.objects)


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
    result.extend(
        (
            f"trailer\n<< /Size {size} /Root {parsed.root_id} 0 R >>\n"
            f"startxref\n{xref_offset}\n%%EOF\n"
        ).encode()
    )
    return bytes(result)
