from __future__ import annotations

import re
import shutil
import sys
from dataclasses import dataclass
from pathlib import Path

from evaluate.multiformat_candidate_fonts import (
    CandidateFontError,
    prepare_font_environment,
    validate_font_bundle,
)
from evaluate.multiformat_candidate_process import (
    CandidateProcessError,
)
from evaluate.multiformat_corpus_sources import validate_source
from evaluate.multiformat_corpus_types import CorpusError, DocumentFormat
from evaluate.multiformat_east_asian_fonts import (
    EastAsianFontError,
    seed_host_profile,
)
from evaluate.multiformat_legacy_process import (
    LegacyProcessRequest,
    LegacyProcessRunner,
    run_checked,
    run_process,
    tool_version,
)
from evaluate.multiformat_legacy_types import (
    LegacyConformanceError,
    LegacyPairJob,
    LegacyPairRuntime,
    LegacyToolIdentity,
)
from evaluate.multiformat_schema import sha256_file
from evaluate.multiformat_subprocess import clean_subprocess_environment

_PAGE_PATTERN = re.compile(r"^Pages:\s+([0-9]+)\s*$", re.MULTILINE)
_FILTERS = {
    DocumentFormat.DOC: ("docx", "doc:MS Word 97"),
    DocumentFormat.XLS: ("xlsx", "xls:MS Excel 97"),
    DocumentFormat.PPT: ("pptx", "ppt:MS PowerPoint 97"),
}


@dataclass(frozen=True, slots=True)
class LegacyExternalTools:
    soffice: Path
    pdfinfo: Path
    font_bundle: Path


@dataclass(frozen=True, slots=True)
class _LegacyMaterializer:
    tools: LegacyExternalTools
    runner: LegacyProcessRunner

    def __call__(self, job: LegacyPairJob) -> int:
        if job.workspace.exists() or job.destination.exists():
            raise LegacyConformanceError("legacy conversion output already exists")
        try:
            modern_extension, output_filter = _FILTERS[job.document_format]
            job.workspace.mkdir(parents=True)
            input_dir = job.workspace / "input"
            binary_dir = job.workspace / "binary"
            pdf_dir = job.workspace / "pdf"
            input_dir.mkdir()
            binary_dir.mkdir()
            pdf_dir.mkdir()
            source = input_dir / f"{job.case_id}.{modern_extension}"
            shutil.copyfile(job.source, source)
            font_environment = prepare_font_environment(
                self.tools.font_bundle,
                job.workspace / "fonts",
            )
            environment = _environment(
                job.workspace,
                font_environment.config_path,
            )
            run_checked(
                self.runner,
                _soffice_request(
                    self.tools.soffice,
                    source,
                    output_filter,
                    binary_dir,
                    job.workspace / "profile-binary",
                    environment,
                    job.workspace / "binary",
                ),
                "LibreOffice conversion failed",
            )
            binary = binary_dir / f"{job.case_id}.{job.document_format.value}"
            _validate_output(binary, job.document_format)
            run_checked(
                self.runner,
                _soffice_request(
                    self.tools.soffice,
                    binary,
                    "pdf",
                    pdf_dir,
                    job.workspace / "profile-pdf",
                    environment,
                    job.workspace / "pdf",
                ),
                "LibreOffice PDF inspection export failed",
            )
            pdf = pdf_dir / f"{job.case_id}.pdf"
            _validate_output(pdf, DocumentFormat.PDF)
            page_request = LegacyProcessRequest(
                (self.tools.pdfinfo.as_posix(), pdf.as_posix()),
                job.workspace,
                environment,
                job.workspace / "pdfinfo.stdout",
                job.workspace / "pdfinfo.stderr",
                30.0,
            )
            run_checked(
                self.runner,
                page_request,
                "PDF unit inspection failed",
            )
            _validate_pdf_pages(page_request.stdout_path)
            job.destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(binary, job.destination)
            return 1
        except LegacyConformanceError:
            raise
        except (
            CandidateFontError,
            CandidateProcessError,
            CorpusError,
            EastAsianFontError,
            KeyError,
            OSError,
            UnicodeError,
        ) as error:
            raise LegacyConformanceError("legacy conversion failed") from error
        finally:
            if job.workspace.exists():
                active = sys.exception()
                try:
                    shutil.rmtree(job.workspace)
                except OSError as error:
                    failure = LegacyConformanceError("legacy workspace cleanup failed")
                    if active is None:
                        raise failure from error
                    active.add_note(str(failure))
                    active.add_note(str(error))


def build_legacy_runtime(
    tools: LegacyExternalTools,
    *,
    runner: LegacyProcessRunner | None = None,
) -> LegacyPairRuntime:
    process_runner = runner or run_process
    try:
        resolved = LegacyExternalTools(
            tools.soffice.resolve(strict=True),
            tools.pdfinfo.resolve(strict=True),
            tools.font_bundle.resolve(strict=True),
        )
        font_hash = validate_font_bundle(resolved.font_bundle)
        identity = LegacyToolIdentity(
            soffice_sha256=sha256_file(resolved.soffice),
            soffice_version=tool_version(
                resolved.soffice,
                ("--version",),
                process_runner,
            ),
            pdfinfo_sha256=sha256_file(resolved.pdfinfo),
            pdfinfo_version=tool_version(
                resolved.pdfinfo,
                ("-v",),
                process_runner,
            ),
            font_environment_sha256=font_hash,
        )
        return LegacyPairRuntime(
            _LegacyMaterializer(resolved, process_runner),
            identity,
        )
    except LegacyConformanceError:
        raise
    except (
        CandidateFontError,
        CandidateProcessError,
        OSError,
        UnicodeError,
    ) as error:
        raise LegacyConformanceError("legacy tool lock failed") from error


def _soffice_request(
    soffice: Path,
    source: Path,
    output_filter: str,
    output_dir: Path,
    profile: Path,
    environment: dict[str, str],
    log_prefix: Path,
) -> LegacyProcessRequest:
    profile.mkdir()
    # ``FONTCONFIG_FILE`` below is honoured on Linux but ignored by the macOS
    # CoreText backend, so corpus fixtures need the same replacement table the
    # reference producer and the shipped converter use.
    _ = seed_host_profile(profile)
    return LegacyProcessRequest(
        (
            soffice.as_posix(),
            f"-env:UserInstallation={profile.as_uri()}",
            "--headless",
            "--nologo",
            "--nolockcheck",
            "--nodefault",
            "--nofirststartwizard",
            "--convert-to",
            output_filter,
            "--outdir",
            output_dir.as_posix(),
            source.as_posix(),
        ),
        source.parent,
        environment,
        log_prefix.with_suffix(".stdout"),
        log_prefix.with_suffix(".stderr"),
        120.0,
    )


def _environment(workspace: Path, font_config: Path) -> dict[str, str]:
    home = workspace / "home"
    home.mkdir()
    return {
        **clean_subprocess_environment(),
        "FONTCONFIG_FILE": font_config.as_posix(),
        "HOME": home.as_posix(),
        "LANG": "C",
        "LC_ALL": "C",
        "TZ": "UTC",
    }


def _validate_output(path: Path, document_format: DocumentFormat) -> None:
    if not path.is_file() or path.is_symlink():
        raise LegacyConformanceError("legacy conversion output is missing")
    validate_source(
        {
            "id": f"runtime-{document_format.value}",
            "path": path.name,
            "sha256": sha256_file(path),
        },
        path.parent,
        document_format,
        require_valid_format=True,
    )


def _validate_pdf_pages(path: Path) -> None:
    value = path.read_text(encoding="utf-8")
    match = _PAGE_PATTERN.search(value)
    if match is None or int(match.group(1)) <= 0:
        raise LegacyConformanceError("PDF unit inspection failed")
