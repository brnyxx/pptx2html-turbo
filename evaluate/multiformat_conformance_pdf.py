"""Canonical PDF form: identity of the canonicalizer and its entry point.

The pipeline runs one way. `multiformat_pdf_types` is the dependency leaf that
defines the parsed-document types, the PDF name grammar, and the structural
constants. Each stage - the xref parser, the Type1 normalizer, the metadata
writer, and the object renumberer - depends on that leaf and never on this
module, so the import graph is acyclic and every stage is imported statically.

This module owns only the canonicalizer's identity and the ordering of those
stages. The leaf names are re-exported so callers keep one import site for the
canonical PDF vocabulary.
"""

from __future__ import annotations

import hashlib
from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path
from typing import Final

from evaluate.multiformat_pdf_order import renumber_pdf_objects
from evaluate.multiformat_pdf_type1 import canonicalize_type1_font_objects
from evaluate.multiformat_pdf_types import (
    PDF_CATALOG_KEYS,
    PDF_DELIMITERS,
    PDF_METADATA_KEYS,
    PDF_OBJECT_STREAM_KEYS,
    PDF_TRAILER_KEYS,
    PDF_TYPE1_KEYS,
    PDF_XREF_STREAM_KEYS,
    ParsedPdfObjects,
    PdfConformanceError,
    PdfNameToken,
    PdfUnsupportedConstructError,
    pdf_decoded_name,
    pdf_literal_end,
    pdf_name_value,
    pdf_reject_unsupported_fields,
    pdf_require_name_value,
    pdf_space_comment_end,
    pdf_top_level_names,
    pdf_trailer_identity,
    pdf_unique_name_tokens,
    pdf_unique_name_values,
)
from evaluate.multiformat_pdf_writer import (
    canonicalize_pdf_metadata,
    write_pdf_objects,
)
from evaluate.multiformat_pdf_xref import parse_pdf_objects

__all__ = [
    "PDF_CANONICALIZER_VERSION",
    "PDF_CATALOG_KEYS",
    "PDF_DELIMITERS",
    "PDF_METADATA_KEYS",
    "PDF_OBJECT_STREAM_KEYS",
    "PDF_TRAILER_KEYS",
    "PDF_TYPE1_KEYS",
    "PDF_XREF_STREAM_KEYS",
    "PageCounter",
    "ParsedPdfObjects",
    "PdfCanonicalizer",
    "PdfCanonicalizerIdentity",
    "PdfConformanceError",
    "PdfConverter",
    "PdfNameToken",
    "PdfUnsupportedConstructError",
    "canonicalize_pdf_bytes",
    "canonicalizer_implementation_sha256",
    "canonicalizer_sources",
    "pdf_canonicalizer_identity",
    "pdf_decoded_name",
    "pdf_literal_end",
    "pdf_name_value",
    "pdf_reject_unsupported_fields",
    "pdf_require_name_value",
    "pdf_space_comment_end",
    "pdf_top_level_names",
    "pdf_trailer_identity",
    "pdf_unique_name_tokens",
    "pdf_unique_name_values",
]

# Version 2 adds canonical indirect-object numbering: objects are renumbered by
# a structural walk from the trailer Root, so a reference export no longer
# carries the object ids its run happened to assign.
PDF_CANONICALIZER_VERSION: Final = "2"
_CANONICALIZER_MODULES: Final = (
    "multiformat_conformance_pdf.py",
    "multiformat_pdf_types.py",
    "multiformat_pdf_xref.py",
    "multiformat_pdf_writer.py",
    "multiformat_pdf_type1.py",
    "multiformat_pdf_order.py",
)
PdfConverter = Callable[[tuple[Path, ...], Path, Path], None]
PdfCanonicalizer = Callable[[Path, Path], None]
PageCounter = Callable[[Path], int]


@dataclass(frozen=True, slots=True)
class PdfCanonicalizerIdentity:
    version: str
    implementation_sha256: str


def canonicalizer_sources() -> tuple[tuple[str, bytes], ...]:
    root = Path(__file__).parent
    return tuple((name, (root / name).read_bytes()) for name in _CANONICALIZER_MODULES)


def canonicalizer_implementation_sha256(
    sources: tuple[tuple[str, bytes], ...],
) -> str:
    digest = hashlib.sha256()
    for name, value in sources:
        encoded_name = name.encode("utf-8")
        digest.update(len(encoded_name).to_bytes(4, "big"))
        digest.update(encoded_name)
        digest.update(len(value).to_bytes(8, "big"))
        digest.update(value)
    return digest.hexdigest()


def pdf_canonicalizer_identity() -> PdfCanonicalizerIdentity:
    return PdfCanonicalizerIdentity(
        PDF_CANONICALIZER_VERSION,
        canonicalizer_implementation_sha256(canonicalizer_sources()),
    )


def canonicalize_pdf_bytes(value: bytes) -> bytes:
    try:
        parsed = parse_pdf_objects(value)
        objects = canonicalize_type1_font_objects(parsed.objects)
        parsed, objects = canonicalize_pdf_metadata(parsed, objects)
        parsed, objects = renumber_pdf_objects(parsed, objects)
        canonical = write_pdf_objects(parsed, objects)
        parse_pdf_objects(canonical)
    except PdfConformanceError:
        raise
    except (IndexError, KeyError, OverflowError, ValueError) as error:
        raise PdfConformanceError("PDF structure is malformed") from error
    return canonical
