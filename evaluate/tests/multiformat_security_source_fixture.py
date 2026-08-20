from __future__ import annotations

from pathlib import Path

from evaluate.tests.multiformat_security_cfb_fixture import (
    write_cfb_security_fixture,
)
from evaluate.tests.multiformat_security_ooxml_fixture import (
    write_ooxml_security_fixture,
)
from evaluate.tests.multiformat_security_pdf_fixture import (
    write_pdf_security_fixture,
)
from evaluate.tests.multiformat_source_fixture import SourceFixtureError

OOXML_FORMATS = frozenset({"docx", "xlsx", "pptx"})
CFB_FORMATS = frozenset({"doc", "xls", "ppt"})


def write_security_source(
    path: Path,
    document_format: str,
    family: str,
) -> None:
    if document_format in OOXML_FORMATS:
        write_ooxml_security_fixture(path, document_format, family)
        return
    if document_format in CFB_FORMATS:
        write_cfb_security_fixture(path, document_format, family)
        return
    if document_format == "pdf":
        write_pdf_security_fixture(path, family)
        return
    raise SourceFixtureError(f"unsupported security fixture format: {document_format}")
