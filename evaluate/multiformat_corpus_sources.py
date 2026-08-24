from __future__ import annotations

import re
import stat
from pathlib import Path
from typing import Final, assert_never

from evaluate.multiformat_cfb import cfb_root_streams
from evaluate.multiformat_corpus_source_fs import (
    FileIdentity,
    descriptor_path,
    rewind_descriptor,
    stable_source_descriptor,
)
from evaluate.multiformat_corpus_types import (
    CorpusError,
    DocumentFormat,
    SourceRecord,
)
from evaluate.multiformat_package_validation import valid_ooxml
from evaluate.multiformat_pdf import valid_pdf
from evaluate.multiformat_schema import JsonValue, sha256_value, string_value

IDENTIFIER: Final[re.Pattern[str]] = re.compile(r"^[a-z0-9][a-z0-9._-]{0,127}$")
LEGACY_STREAMS: Final[frozenset[str]] = frozenset(
    {"WordDocument", "Workbook", "Book", "PowerPoint Document"}
)


def validate_source(
    item: dict[str, JsonValue],
    root: Path,
    document_format: DocumentFormat,
    *,
    require_valid_format: bool,
) -> SourceRecord:
    record, _ = _validate_source_with_binding(
        item,
        root,
        document_format,
        require_valid_format=require_valid_format,
    )
    return record


def _validate_source_with_binding(
    item: dict[str, JsonValue],
    root: Path,
    document_format: DocumentFormat,
    *,
    require_valid_format: bool,
) -> tuple[SourceRecord, FileIdentity]:
    item_id = string_value(item, "id")
    validate_identifier(item_id, "source.id")
    relative_path = string_value(item, "path")
    expected_digest = sha256_value(item, "sha256")
    source_path = resolve_source_path(root, relative_path)
    if source_path.suffix.lower() != f".{document_format.value}":
        raise CorpusError("source.format", relative_path)
    with stable_source_descriptor(source_path, relative_path) as opened:
        if opened.digest != expected_digest:
            raise CorpusError("source.sha256", relative_path)
        if require_valid_format:
            rewind_descriptor(opened.descriptor, relative_path)
        if require_valid_format and not _matches_format(
            descriptor_path(opened.descriptor),
            document_format,
        ):
            raise CorpusError("source.format", relative_path)
        _after_source_validation(source_path)
        _before_source_final_verification(source_path)
        identity = opened.identity
    return SourceRecord(item_id, relative_path, expected_digest), identity


def _after_source_validation(_path: Path) -> None:
    """Deterministic race-test seam; production performs no action."""


def _before_source_final_verification(_path: Path) -> None:
    """Deterministic final-boundary race-test seam; production performs no action."""


def validate_identifier(value: str, reason: str) -> None:
    if IDENTIFIER.fullmatch(value) is None:
        raise CorpusError(reason, value)


def resolve_source_path(root: Path, relative_path: str) -> Path:
    relative = Path(relative_path)
    if (
        relative.is_absolute()
        or "\\" in relative_path
        or relative_path != relative.as_posix()
        or any(part in {"", ".", ".."} for part in relative.parts)
    ):
        raise CorpusError("source.path", relative_path)
    try:
        root_stat = root.lstat()
    except OSError as error:
        raise CorpusError("source.path", relative_path) from error
    if stat.S_ISLNK(root_stat.st_mode) or not stat.S_ISDIR(root_stat.st_mode):
        raise CorpusError("source.path", relative_path)
    resolved_root = root.resolve(strict=True)
    candidate = resolved_root
    for part in relative.parts:
        candidate /= part
        if candidate.is_symlink():
            raise CorpusError("source.path", relative_path)
    try:
        candidate = candidate.resolve(strict=True)
    except OSError as error:
        raise CorpusError("source.path", relative_path) from error
    if not candidate.is_relative_to(resolved_root) or not candidate.is_file():
        raise CorpusError("source.path", relative_path)
    return candidate


def _matches_format(path: Path, document_format: DocumentFormat) -> bool:
    match document_format:
        case DocumentFormat.DOCX:
            return valid_ooxml(
                path,
                "word/document.xml",
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml",
                "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}document",
            )
        case DocumentFormat.XLSX:
            return valid_ooxml(
                path,
                "xl/workbook.xml",
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml",
                "{http://schemas.openxmlformats.org/spreadsheetml/2006/main}workbook",
            )
        case DocumentFormat.PPTX:
            return valid_ooxml(
                path,
                "ppt/presentation.xml",
                "application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml",
                "{http://schemas.openxmlformats.org/presentationml/2006/main}presentation",
            )
        case DocumentFormat.DOC:
            return _has_cfb_stream(path, "WordDocument")
        case DocumentFormat.XLS:
            return _has_cfb_stream(path, "Workbook", "Book")
        case DocumentFormat.PPT:
            return _has_cfb_stream(path, "PowerPoint Document")
        case DocumentFormat.PDF:
            return valid_pdf(path)
        case _ as unreachable:
            assert_never(unreachable)


def _has_cfb_stream(path: Path, *expected: str) -> bool:
    streams = cfb_root_streams(path)
    if streams is None:
        return False
    markers = set(streams) & LEGACY_STREAMS
    return (
        len(markers) == 1
        and any(name in markers for name in expected)
        and all(streams[name] > 0 for name in markers)
    )
