from __future__ import annotations

import os
import stat
from pathlib import Path
from typing import assert_never

from evaluate.multiformat_corpus_sources import validate_identifier
from evaluate.multiformat_corpus_types import CorpusError, DocumentFormat
from evaluate.multiformat_security_source_cfb import (
    write_cfb_security_fixture,
)
from evaluate.multiformat_security_source_ooxml import (
    write_ooxml_security_fixture,
)
from evaluate.multiformat_security_source_pdf import (
    write_pdf_security_fixture,
)
from evaluate.multiformat_source_fixture import SourceFixtureError


def write_security_source(
    path: Path,
    document_format: DocumentFormat,
    family: str,
) -> None:
    if not isinstance(document_format, DocumentFormat):
        raise SourceFixtureError("security fixture format must be typed")
    try:
        validate_identifier(family, "security.fixture.family")
    except CorpusError as error:
        raise SourceFixtureError(
            f"invalid security fixture family: {family}"
        ) from error
    identity = _claim_destination(path)
    try:
        _write_claimed_source(path, document_format, family)
        if not _owned_regular_file(path, identity):
            raise SourceFixtureError("security fixture destination identity changed")
    except SourceFixtureError:
        _unlink_owned(path, identity)
        raise
    except OSError as error:
        _unlink_owned(path, identity)
        raise SourceFixtureError("security fixture write failed") from error


def _claim_destination(path: Path) -> tuple[int, int]:
    flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL
    flags |= getattr(os, "O_CLOEXEC", 0) | getattr(os, "O_NOFOLLOW", 0)
    try:
        descriptor = os.open(path, flags, 0o600)
    except OSError as error:
        raise SourceFixtureError("security fixture destination unavailable") from error
    try:
        value = os.fstat(descriptor)
        return value.st_dev, value.st_ino
    finally:
        os.close(descriptor)


def _write_claimed_source(
    path: Path,
    document_format: DocumentFormat,
    family: str,
) -> None:
    match document_format:
        case DocumentFormat.DOCX | DocumentFormat.XLSX | DocumentFormat.PPTX:
            write_ooxml_security_fixture(path, document_format.value, family)
        case DocumentFormat.DOC | DocumentFormat.XLS | DocumentFormat.PPT:
            write_cfb_security_fixture(path, document_format.value, family)
        case DocumentFormat.PDF:
            write_pdf_security_fixture(path, family)
        case unreachable:
            assert_never(unreachable)


def _owned_regular_file(path: Path, identity: tuple[int, int]) -> bool:
    try:
        value = path.lstat()
    except FileNotFoundError:
        return False
    return (
        stat.S_ISREG(value.st_mode)
        and value.st_dev == identity[0]
        and value.st_ino == identity[1]
    )


def _unlink_owned(path: Path, identity: tuple[int, int]) -> None:
    if _owned_regular_file(path, identity):
        path.unlink()
