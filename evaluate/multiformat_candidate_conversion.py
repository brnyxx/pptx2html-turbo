from __future__ import annotations

import hashlib
import json
import os
import shutil
import xml.etree.ElementTree as ET
import zipfile
from dataclasses import dataclass
from pathlib import Path

from evaluate.multiformat_candidate_process import (
    CandidateProcessError,
    run_bounded_process,
)
from evaluate.multiformat_candidate_types import CandidateCaptureError
from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_schema import JsonValue

MAX_HTML_BYTES = 64 * 1024 * 1024
MAX_LOG_BYTES = 8 * 1024 * 1024


class CandidateConversionError(CandidateCaptureError):
    pass


@dataclass(frozen=True, slots=True)
class ConversionResult:
    html: str
    html_path: Path
    diagnostics: Path
    source_sha256: str
    command: tuple[str, ...]


def run_conversion(
    converter: Path,
    source: Path,
    document_format: DocumentFormat,
    output_dir: Path,
    *,
    soffice: Path,
    pdftohtml: Path,
    pdfinfo: Path,
    timeout_seconds: float,
) -> ConversionResult:
    if output_dir.exists() and any(output_dir.iterdir()):
        raise CandidateConversionError(f"nonempty conversion output: {output_dir}")
    output_dir.mkdir(parents=True, exist_ok=True)
    source_hash = _sha256(source)
    staged_source = output_dir / f"source.{document_format.value}"
    shutil.copyfile(source, staged_source)
    if _sha256(staged_source) != source_hash:
        raise CandidateConversionError("staged source hash mismatch")
    html_path = output_dir / "document.html"
    diagnostics = output_dir / "diagnostics.json"
    command = (
        _executable(converter),
        staged_source.as_posix(),
        "--input-format",
        document_format.value,
        "--output",
        html_path.as_posix(),
        "--diagnostics",
        diagnostics.as_posix(),
        *_presentation_args(staged_source, document_format),
        "--soffice",
        _executable(soffice),
        "--pdftohtml",
        _executable(pdftohtml),
        "--pdfinfo",
        _executable(pdfinfo),
    )
    stdout_path = output_dir / "stdout.log"
    stderr_path = output_dir / "stderr.log"
    home = output_dir / "home"
    temporary = output_dir / "tmp"
    home.mkdir()
    temporary.mkdir()
    environment = {
        "HOME": home.as_posix(),
        "TMPDIR": temporary.as_posix(),
        "PATH": os.defpath,
        "TZ": "UTC",
        "LANG": "C.UTF-8",
        "LC_ALL": "C.UTF-8",
        "SAL_USE_VCLPLUGIN": "svp",
    }
    try:
        exit_code = run_bounded_process(
            command,
            output_dir,
            environment,
            stdout_path,
            stderr_path,
            timeout_seconds=timeout_seconds,
            max_log_bytes=MAX_LOG_BYTES,
        )
    except CandidateProcessError as error:
        raise CandidateConversionError(str(error)) from error
    if exit_code != 0:
        raise CandidateConversionError(f"converter exit code {exit_code}")
    if _sha256(source) != source_hash or _sha256(staged_source) != source_hash:
        raise CandidateConversionError("source changed during conversion")
    if not html_path.is_file() or not 0 < html_path.stat().st_size <= MAX_HTML_BYTES:
        raise CandidateConversionError("missing or oversized HTML output")
    if (
        not diagnostics.is_file()
        or diagnostics.stat().st_size <= 0
        or diagnostics.stat().st_size > MAX_LOG_BYTES
    ):
        raise CandidateConversionError("missing or oversized diagnostics output")
    try:
        diagnostics_value = json.loads(diagnostics.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise CandidateConversionError("invalid diagnostics output") from error
    _validate_diagnostics(diagnostics_value, document_format)
    try:
        html = html_path.read_text(encoding="utf-8")
    except (OSError, UnicodeError) as error:
        raise CandidateConversionError("HTML output is not UTF-8") from error
    return ConversionResult(
        html,
        html_path,
        diagnostics,
        source_hash,
        command,
    )


def _executable(path: Path) -> str:
    try:
        resolved = path.resolve(strict=True)
    except OSError as error:
        raise CandidateConversionError(f"missing executable: {path}") from error
    if not resolved.is_file() or not os.access(resolved, os.X_OK):
        raise CandidateConversionError(f"not executable: {path}")
    return resolved.as_posix()


def _presentation_args(
    source: Path,
    document_format: DocumentFormat,
) -> tuple[str, ...]:
    if document_format is not DocumentFormat.PPTX:
        return ()
    try:
        with zipfile.ZipFile(source) as archive:
            root = ET.fromstring(archive.read("ppt/presentation.xml"))
        size = root.find(
            "{http://schemas.openxmlformats.org/presentationml/2006/main}sldSz"
        )
        if size is None:
            raise CandidateConversionError("PPTX slide size is missing")
        width = int(size.attrib["cx"])
        height = int(size.attrib["cy"])
    except (
        KeyError,
        OSError,
        ValueError,
        ET.ParseError,
        zipfile.BadZipFile,
    ) as error:
        raise CandidateConversionError("invalid PPTX slide geometry") from error
    if width <= 0 or height <= 0 or abs(width / height - 16 / 9) > 0.000001:
        raise CandidateConversionError("PPTX capture requires 16:9 slide geometry")
    css_width = width * 96 / 914_400
    scale = 960 / css_width
    return "--presentation-scale", f"{scale:.12g}"


def _validate_diagnostics(
    value: JsonValue,
    document_format: DocumentFormat,
) -> None:
    if not isinstance(value, list) or any(not isinstance(item, dict) for item in value):
        raise CandidateConversionError("diagnostics output has an invalid shape")
    codes = {
        item.get("code")
        for item in value
        if isinstance(item.get("code"), str) and item.get("code")
    }
    if (
        document_format is not DocumentFormat.PPTX
        and "NATIVE_BACKEND_OPAQUE" not in codes
    ):
        raise CandidateConversionError("native runtime diagnostics are missing")
    if codes & {
        "NATIVE_NETWORK_ISOLATION_DISABLED",
        "PROCESS_ISOLATION_DISABLED",
        "PROCESS_ISOLATION_UNAVAILABLE",
    }:
        raise CandidateConversionError("native runtime isolation diagnostics failed")
    # A truncated cell scan abandons all coordinate attribution, so the
    # conversion cannot back spreadsheet cell evidence at all.
    if "SPREADSHEET_CELL_SCAN_TRUNCATED" in codes:
        raise CandidateConversionError("spreadsheet cell scan truncated")


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


__all__ = [
    "CandidateConversionError",
    "run_conversion",
]
