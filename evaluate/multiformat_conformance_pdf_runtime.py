from __future__ import annotations

import platform
import re
import subprocess
from pathlib import Path

from evaluate.multiformat_candidate_fonts import (
    CandidateFontError,
    prepare_font_environment,
    validate_font_bundle,
)
from evaluate.multiformat_conformance_pdf import (
    PageCounter,
    PdfCanonicalizer,
    PdfConformanceError,
    PdfConverter,
)
from evaluate.multiformat_schema import JsonValue, sha256_file
from evaluate.multiformat_subprocess import clean_subprocess_environment

PAGES_PATTERN = re.compile(r"^Pages:\s+([0-9]+)\s*$", re.MULTILINE)


def build_pdf_tool_lock(
    soffice: Path,
    pdfinfo: Path,
    pdftocairo: Path,
    font_bundle: Path,
) -> dict[str, JsonValue]:
    try:
        return {
            "soffice_sha256": sha256_file(soffice),
            "soffice_version": _tool_version(soffice, ["--version"]),
            "pdfinfo_sha256": sha256_file(pdfinfo),
            "pdfinfo_version": _tool_version(pdfinfo, ["-v"]),
            "pdftocairo_sha256": sha256_file(pdftocairo),
            "pdftocairo_version": _tool_version(pdftocairo, ["-v"]),
            "font_environment_sha256": validate_font_bundle(font_bundle),
        }
    except (CandidateFontError, OSError, subprocess.SubprocessError) as error:
        raise PdfConformanceError("PDF conformance tool lock failed") from error


def soffice_converter(
    soffice: Path,
    font_bundle: Path,
) -> PdfConverter:
    executable = soffice.resolve(strict=True)

    def convert(
        html_paths: tuple[Path, ...],
        output_dir: Path,
        profile_dir: Path,
    ) -> None:
        environment = clean_subprocess_environment()
        if platform.system() != "Darwin":
            try:
                fonts = prepare_font_environment(
                    font_bundle,
                    profile_dir / "font-runtime",
                )
            except CandidateFontError as error:
                raise PdfConformanceError(
                    "PDF conformance font environment failed"
                ) from error
            environment["FONTCONFIG_FILE"] = fonts.config_path.as_posix()
        environment.update(
            {
                "HOME": (profile_dir / "home").as_posix(),
                "TMPDIR": (profile_dir / "tmp").as_posix(),
            }
        )
        Path(environment["HOME"]).mkdir(parents=True)
        Path(environment["TMPDIR"]).mkdir(parents=True)
        for html_path in html_paths:
            result = subprocess.run(
                [
                    executable.as_posix(),
                    (
                        "-env:UserInstallation="
                        f"{(profile_dir / f'lo-{html_path.stem}').as_uri()}"
                    ),
                    "--headless",
                    "--infilter=HTML (StarWriter)",
                    "--convert-to",
                    "pdf:writer_pdf_Export",
                    "--outdir",
                    output_dir.as_posix(),
                    html_path.as_posix(),
                ],
                check=False,
                capture_output=True,
                env=environment,
                timeout=60,
            )
            if (
                result.returncode != 0
                or len(result.stdout) > 1024 * 1024
                or len(result.stderr) > 1024 * 1024
            ):
                raise PdfConformanceError("LibreOffice PDF conversion failed")

    return convert


def pdf_canonicalizer(pdftocairo: Path) -> PdfCanonicalizer:
    executable = pdftocairo.resolve(strict=True)

    def canonicalize(source: Path, destination: Path) -> None:
        result = subprocess.run(
            [
                executable.as_posix(),
                "-pdf",
                source.as_posix(),
                destination.as_posix(),
            ],
            check=False,
            capture_output=True,
            env=clean_subprocess_environment(),
            timeout=60,
        )
        if (
            result.returncode != 0
            or not destination.is_file()
            or len(result.stdout) > 1024 * 1024
            or len(result.stderr) > 1024 * 1024
        ):
            raise PdfConformanceError("pdftocairo canonicalization failed")

    return canonicalize


def pdf_page_counter(pdfinfo: Path) -> PageCounter:
    executable = pdfinfo.resolve(strict=True)

    def count(path: Path) -> int:
        result = subprocess.run(
            [executable.as_posix(), path.as_posix()],
            check=False,
            capture_output=True,
            env=clean_subprocess_environment(),
            timeout=15,
        )
        if result.returncode != 0 or len(result.stdout) > 1024 * 1024:
            raise PdfConformanceError("pdfinfo failed")
        match = PAGES_PATTERN.search(result.stdout.decode(errors="strict"))
        if match is None:
            raise PdfConformanceError("pdfinfo page count is missing")
        return int(match.group(1))

    return count


def _tool_version(path: Path, arguments: list[str]) -> str:
    result = subprocess.run(
        [path.resolve(strict=True).as_posix(), *arguments],
        check=False,
        capture_output=True,
        env=clean_subprocess_environment(),
        timeout=15,
    )
    value = (result.stdout + result.stderr).decode(errors="strict").strip()
    if result.returncode != 0 or not value or len(value) > 1024 * 1024:
        raise PdfConformanceError("PDF conformance tool version failed")
    return value.splitlines()[0]
