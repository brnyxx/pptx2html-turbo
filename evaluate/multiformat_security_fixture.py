from __future__ import annotations

from pathlib import Path
from typing import assert_never

from evaluate.multiformat_corpus_types import CorpusError, DocumentFormat
from evaluate.multiformat_security_cfb import detect_cfb_security_families
from evaluate.multiformat_security_ooxml import detect_ooxml_security_families
from evaluate.multiformat_security_pdf import detect_pdf_security_families


def validate_security_fixture(
    path: Path,
    document_format: DocumentFormat,
    case_family: str,
) -> None:
    match document_format:
        case DocumentFormat.DOCX | DocumentFormat.XLSX | DocumentFormat.PPTX:
            families = detect_ooxml_security_families(path, document_format)
        case DocumentFormat.DOC | DocumentFormat.XLS | DocumentFormat.PPT:
            families = detect_cfb_security_families(path, document_format)
        case DocumentFormat.PDF:
            families = detect_pdf_security_families(path)
        case unreachable:
            assert_never(unreachable)
    if families != {case_family}:
        detail = ",".join(sorted(families)) if families else "unproved"
        raise CorpusError("security.fixture", f"{case_family}:{detail}")
