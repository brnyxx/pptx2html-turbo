from __future__ import annotations

import argparse
import shutil
import tempfile
from pathlib import Path

from evaluate.build_multiformat_conformance_plan import (
    ConformancePlanError,
    validate_conformance_plan,
)
from evaluate.multiformat_candidate_artifacts import write_canonical_json
from evaluate.multiformat_conformance_pdf import (
    PageCounter,
    PdfCanonicalizer,
    PdfConformanceError,
    PdfConverter,
    pdf_case_html,
)
from evaluate.multiformat_conformance_pdf_runtime import (
    build_pdf_tool_lock,
    pdf_canonicalizer,
    pdf_page_counter,
    soffice_converter,
)
from evaluate.multiformat_corpus_items import object_list
from evaluate.multiformat_corpus_sources import validate_source
from evaluate.multiformat_corpus_types import CorpusError, DocumentFormat
from evaluate.multiformat_pdf_link_annotation import add_link_annotation
from evaluate.multiformat_pdf_writer import canonicalize_pdf_bytes
from evaluate.multiformat_schema import (
    JsonValue,
    object_value,
    sha256_file,
    string_value,
)
from evaluate.multiformat_strict_json import StrictJsonError, read_strict_object


def generate_pdf_conformance(
    contract: Path,
    plan: Path,
    output_dir: Path,
    *,
    converter: PdfConverter,
    canonicalizer: PdfCanonicalizer,
    page_counter: PageCounter,
    tools: dict[str, JsonValue],
) -> Path:
    if output_dir.exists():
        raise PdfConformanceError("PDF conformance output already exists")
    try:
        validate_conformance_plan(contract, plan)
        _validate_tools(tools)
        plan_values = read_strict_object(plan)
        pdf_values = object_value(
            object_value(plan_values, "formats"),
            "pdf",
        )
        cases = object_list(pdf_values, "cases", "pdf.conformance.cases")
        if len(cases) != 100:
            raise PdfConformanceError("PDF conformance requires 100 cases")
        with tempfile.TemporaryDirectory() as temp_dir:
            runtime = Path(temp_dir)
            html_root = runtime / "html"
            raw_root = runtime / "raw"
            profile_root = runtime / "profile"
            html_root.mkdir()
            raw_root.mkdir()
            html_paths = []
            input_hashes: dict[str, str] = {}
            for case in cases:
                case_id = string_value(case, "id")
                html_path = html_root / f"{case_id}.html"
                html_path.write_bytes(pdf_case_html(case))
                html_paths.append(html_path)
                input_hashes[case_id] = sha256_file(html_path)
            converter(tuple(html_paths), raw_root, profile_root)
            output_dir.mkdir(parents=True)
            source_root = output_dir / "sources" / "pdf"
            source_root.mkdir(parents=True)
            files = _materialize_sources(
                cases,
                raw_root,
                source_root,
                output_dir,
                canonicalizer,
                page_counter,
                input_hashes,
            )
        manifest = output_dir / "generation-manifest.json"
        write_canonical_json(
            manifest,
            {
                "schema_version": 1,
                "status": "GENERATED",
                "format": "pdf",
                "contract_sha256": sha256_file(contract),
                "plan_sha256": sha256_file(plan),
                "tools": tools,
                "files": files,
            },
        )
        _validate_output_set(output_dir, files)
        return manifest
    except PdfConformanceError:
        if output_dir.exists():
            shutil.rmtree(output_dir)
        raise
    except (
        ConformancePlanError,
        CorpusError,
        OSError,
        StrictJsonError,
        TypeError,
        ValueError,
    ) as error:
        if output_dir.exists():
            shutil.rmtree(output_dir)
        raise PdfConformanceError("PDF conformance generation failed") from error


def _materialize_sources(
    cases: list[dict[str, JsonValue]],
    raw_root: Path,
    source_root: Path,
    evidence_root: Path,
    canonicalizer: PdfCanonicalizer,
    page_counter: PageCounter,
    input_hashes: dict[str, str],
) -> list[dict[str, JsonValue]]:
    files: list[dict[str, JsonValue]] = []
    for case in cases:
        case_id = string_value(case, "id")
        source = source_root / f"{case_id}.pdf"
        raw = raw_root / source.name
        canonical = raw_root / f"{case_id}-canonical.pdf"
        canonicalizer(raw, canonical)
        value = canonicalize_pdf_bytes(canonical.read_bytes())
        if string_value(case, "primary_stratum") == "forms-annotations-links":
            value = add_link_annotation(value)
        source.write_bytes(value)
        digest = sha256_file(source)
        validate_source(
            {
                "id": case_id,
                "path": source.relative_to(evidence_root).as_posix(),
                "sha256": digest,
            },
            evidence_root,
            DocumentFormat.PDF,
            require_valid_format=True,
        )
        if page_counter(source) != 1:
            raise PdfConformanceError("PDF conformance source is not one page")
        files.append(
            {
                "id": case_id,
                "primary_stratum": string_value(case, "primary_stratum"),
                "path": source.relative_to(evidence_root).as_posix(),
                "sha256": digest,
                "input_sha256": input_hashes[case_id],
                "unit_count": 1,
            }
        )
    return files


def _validate_tools(tools: dict[str, JsonValue]) -> None:
    required = {
        "soffice_sha256",
        "soffice_version",
        "pdfinfo_sha256",
        "pdfinfo_version",
        "pdftocairo_sha256",
        "pdftocairo_version",
        "font_environment_sha256",
    }
    if set(tools) != required or any(
        not isinstance(tools[field], str) or not tools[field] for field in required
    ):
        raise PdfConformanceError("PDF conformance tool lock is invalid")


def _validate_output_set(
    root: Path,
    files: list[dict[str, JsonValue]],
) -> None:
    expected = {
        root / "generation-manifest.json",
        *(root / string_value(item, "path") for item in files),
    }
    actual = {path for path in root.rglob("*") if path.is_file() or path.is_symlink()}
    if actual != expected:
        raise PdfConformanceError("PDF conformance file set differs")


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Materialize a one-page PDF conformance snapshot.",
    )
    parser.add_argument("--contract", type=Path, required=True)
    parser.add_argument("--plan", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--soffice", type=Path, required=True)
    parser.add_argument("--pdfinfo", type=Path, required=True)
    parser.add_argument("--pdftocairo", type=Path, required=True)
    parser.add_argument("--font-bundle", type=Path, required=True)
    arguments = parser.parse_args()
    try:
        tools = build_pdf_tool_lock(
            arguments.soffice,
            arguments.pdfinfo,
            arguments.pdftocairo,
            arguments.font_bundle,
        )
        generate_pdf_conformance(
            arguments.contract,
            arguments.plan,
            arguments.output_dir,
            converter=soffice_converter(
                arguments.soffice,
                arguments.font_bundle,
            ),
            canonicalizer=pdf_canonicalizer(arguments.pdftocairo),
            page_counter=pdf_page_counter(arguments.pdfinfo),
            tools=tools,
        )
    except PdfConformanceError as error:
        parser.error(str(error))


if __name__ == "__main__":
    main()
