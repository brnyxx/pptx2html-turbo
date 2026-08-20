from typing import Literal, Optional

DocumentFormat = Literal["pptx", "docx", "doc", "xlsx", "xls", "ppt", "pdf"]
UnitKind = Literal["page", "sheet-page", "slide", "slide-page"]

class DocumentConversionResult:
    html: str
    format: DocumentFormat
    unit_count: int
    unit_kind: UnitKind
    backend_name: str
    backend_version: str
    diagnostics_json: str

def detect_format(data: bytes, filename: Optional[str] = None) -> DocumentFormat:
    """Detect a supported document format from bytes and an optional filename."""
    ...

def convert_file(
    path: str,
    *,
    allow_unisolated: bool = False,
) -> DocumentConversionResult:
    """Convert a supported Office document or PDF file to embedded-asset HTML."""
    ...

def convert_bytes(
    data: bytes,
    filename: Optional[str] = None,
    *,
    allow_unisolated: bool = False,
) -> DocumentConversionResult:
    """Convert supported document bytes to embedded-asset HTML."""
    ...

def supported_formats() -> list[DocumentFormat]:
    """Return every format recognized by the universal API."""
    ...
