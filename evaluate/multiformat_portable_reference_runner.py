from __future__ import annotations

import platform
import shutil
from dataclasses import dataclass
from pathlib import Path

from evaluate.multiformat_candidate_artifacts import write_canonical_json
from evaluate.multiformat_candidate_process import (
    CandidateProcessError,
    run_bounded_process,
)
from evaluate.multiformat_candidate_sources import CandidateSource
from evaluate.multiformat_conformance_pdf import (
    PdfConformanceError,
    canonicalize_pdf_bytes,
)
from evaluate.multiformat_office_oracle_batch import OfficeOracleBatchFile
from evaluate.multiformat_portable_spreadsheet import extract_xlsx_semantics
from evaluate.multiformat_portable_reference_outputs import (
    PortableReferenceOutputError,
    executable as _executable,
    expand as _expand,
    page_count as _page_count,
    page_images as _page_images,
)
from evaluate.multiformat_reference_routing import (
    DocumentFormat,
    RoutingIdentity,
    ToolRole,
)
from evaluate.multiformat_schema import sha256_file

MAX_LOG_BYTES = 8 * 1024 * 1024


class PortableReferenceRunError(ValueError):
    pass


class PortableReferenceIncompleteError(PortableReferenceRunError):
    pass


@dataclass(frozen=True, slots=True)
class PortableReferenceTools:
    libreoffice: Path
    poppler_metadata: Path
    poppler_render: Path
    poppler_text: Path
    sandbox_exec: Path
    sandbox_profile: Path

    def path_for(self, role: ToolRole) -> Path:
        return {
            ToolRole.LIBREOFFICE: self.libreoffice,
            ToolRole.POPPLER_METADATA: self.poppler_metadata,
            ToolRole.POPPLER_RENDER: self.poppler_render,
            ToolRole.POPPLER_TEXT: self.poppler_text,
        }[role]


def run_reference_source(
    source: CandidateSource,
    document_format: DocumentFormat,
    routing: RoutingIdentity,
    tools: PortableReferenceTools,
    output_dir: Path,
) -> OfficeOracleBatchFile:
    """Execute one frozen route and return inventory-compatible raw artifacts."""
    if output_dir.exists():
        raise PortableReferenceRunError("portable reference output already exists")
    output_dir.mkdir(parents=True)
    original_digest = sha256_file(source.path)
    if original_digest != source.source_sha256:
        raise PortableReferenceRunError("portable reference source drifted")
    staged = output_dir / f"source.{document_format.value}"
    shutil.copyfile(source.path, staged)
    profile = output_dir / "profile"
    home = output_dir / "home"
    profile.mkdir()
    home.mkdir()
    route = next(
        (item for item in routing.routes if item.format.value == document_format.value),
        None,
    )
    if route is None:
        raise PortableReferenceRunError("portable reference route is missing")
    reference_pdf = output_dir / "reference.pdf"
    values = {
        "source": staged.as_posix(),
        "output_dir": output_dir.as_posix(),
        "profile_uri": profile.resolve().as_uri(),
        "reference_pdf": reference_pdf.as_posix(),
        "render_prefix": (output_dir / "page").as_posix(),
        "text_output": (output_dir / "text-layout.html").as_posix(),
    }
    environment = {
        "HOME": home.as_posix(),
        "LANG": "C.UTF-8",
        "LC_ALL": "C.UTF-8",
        "TZ": "UTC",
    }
    try:
        for index, command in enumerate(route.commands, start=1):
            executable = _executable(tools.path_for(command.tool_role))
            arguments = tuple(
                _expand(argument, values) for argument in command.arguments
            )
            _run(
                _sandbox(tools, (executable, *arguments)),
                output_dir,
                environment,
                index,
                command.timeout_seconds,
            )
            if command.tool_role is ToolRole.POPPLER_METADATA:
                shutil.copyfile(
                    output_dir / f"command-{index}.stdout", output_dir / "pdfinfo.txt"
                )
            if command.tool_role is ToolRole.LIBREOFFICE:
                generated = output_dir / "source.pdf"
                if not generated.is_file():
                    raise PortableReferenceRunError(
                        "LibreOffice emitted no reference PDF"
                    )
                canonical = canonicalize_pdf_bytes(generated.read_bytes())
                reference_pdf.write_bytes(canonical)
                generated.unlink()
            _after_command(source.path)
            if (
                sha256_file(source.path) != original_digest
                or sha256_file(staged) != original_digest
            ):
                raise PortableReferenceRunError("portable reference source drifted")
    except (CandidateProcessError, OSError, PdfConformanceError) as error:
        raise PortableReferenceRunError("portable reference command failed") from error
    normative_pdf = staged if document_format is DocumentFormat.PDF else reference_pdf
    if not normative_pdf.is_file() or normative_pdf.stat().st_size == 0:
        raise PortableReferenceRunError("portable reference PDF is missing")
    try:
        pages = _page_count(output_dir / "pdfinfo.txt")
        images = _page_images(output_dir, pages, document_format)
    except PortableReferenceOutputError as error:
        raise PortableReferenceRunError(str(error)) from error
    semantic = output_dir / "semantic.json"
    if document_format is DocumentFormat.XLSX:
        write_canonical_json(semantic, extract_xlsx_semantics(staged))
    elif document_format is DocumentFormat.XLS:
        xlsx = _convert_xls_semantics(staged, output_dir, profile, environment, tools)
        write_canonical_json(semantic, extract_xlsx_semantics(xlsx))
    else:
        write_canonical_json(semantic, {})
    return OfficeOracleBatchFile(
        source.source_id,
        document_format.value,
        original_digest,
        normative_pdf,
        semantic,
        output_dir / "text-layout.html",
        tuple(images),
    )


def _convert_xls_semantics(
    source: Path,
    root: Path,
    profile: Path,
    environment: dict[str, str],
    tools: PortableReferenceTools,
) -> Path:
    target = root / "xlsx-semantic"
    target.mkdir()
    command = (
        _executable(tools.libreoffice),
        "--headless",
        "--nologo",
        "--nodefault",
        "--nolockcheck",
        "--nofirststartwizard",
        f"-env:UserInstallation={profile.resolve().as_uri()}",
        "--convert-to",
        "xlsx",
        "--outdir",
        target.as_posix(),
        source.as_posix(),
    )
    _run(_sandbox(tools, command), root, environment, 5, 120)
    output = target / "source.xlsx"
    if not output.is_file():
        raise PortableReferenceRunError("LibreOffice emitted no semantic XLSX")
    return output


def _sandbox(
    tools: PortableReferenceTools, command: tuple[str, ...]
) -> tuple[str, ...]:
    if platform.system() != "Darwin":
        raise PortableReferenceIncompleteError(
            "portable network sandbox is unsupported on this host"
        )
    executable = _executable(tools.sandbox_exec)
    profile = tools.sandbox_profile.resolve(strict=True)
    if (
        profile.read_text(encoding="utf-8")
        != "(version 1)\n(allow default)\n(deny network*)\n"
    ):
        raise PortableReferenceRunError("portable sandbox profile differs")
    return executable, "-f", profile.as_posix(), *command


def _run(
    command: tuple[str, ...],
    cwd: Path,
    environment: dict[str, str],
    index: int,
    timeout: int,
) -> None:
    result = run_bounded_process(
        command,
        cwd,
        environment,
        cwd / f"command-{index}.stdout",
        cwd / f"command-{index}.stderr",
        timeout_seconds=timeout,
        max_log_bytes=MAX_LOG_BYTES,
    )
    if result != 0:
        raise PortableReferenceRunError(f"portable reference command {index} failed")


def _after_command(_source: Path) -> None:
    """Deterministic source-drift test seam."""
