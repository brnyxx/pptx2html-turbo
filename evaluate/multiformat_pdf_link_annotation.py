from __future__ import annotations

import re

from evaluate.multiformat_pdf_types import PdfConformanceError
from evaluate.multiformat_pdf_writer import write_pdf_objects
from evaluate.multiformat_pdf_xref import parse_pdf_objects


def add_link_annotation(value: bytes) -> bytes:
    parsed = parse_pdf_objects(value)
    objects = dict(parsed.objects)
    page_id = _page_id(objects)
    annotation_id = max(objects) + 1
    objects[page_id] = _add_page_reference(
        objects[page_id],
        annotation_id,
    )
    objects[annotation_id] = (
        f"{annotation_id} 0 obj\n".encode()
        + b"<< /Type /Annot /Subtype /Link /Rect [72 72 360 108] "
        + b"/Border [0 0 1] /A << /S /URI "
        + b"/URI (https://example.com/conformance) >> >>\nendobj"
    )
    return write_pdf_objects(parsed, objects)


def _page_id(objects: dict[int, bytes]) -> int:
    pages = [
        object_id
        for object_id, value in objects.items()
        if re.search(rb"/Type\s*/Page(?!s)", value)
    ]
    if len(pages) != 1:
        raise PdfConformanceError("PDF must contain exactly one page")
    return pages[0]


def _add_page_reference(value: bytes, annotation_id: int) -> bytes:
    if b"/Annots" in value:
        raise PdfConformanceError("PDF page already has annotations")
    dictionary_end = value.rfind(b">>")
    if dictionary_end < 0:
        raise PdfConformanceError("PDF page dictionary is invalid")
    return (
        value[:dictionary_end]
        + f"/Annots [{annotation_id} 0 R]\n>>".encode()
        + value[dictionary_end + 2 :]
    )
