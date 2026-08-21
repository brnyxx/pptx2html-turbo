from __future__ import annotations

import argparse
import hashlib
import json
import shutil
from pathlib import Path
from typing import Final

from evaluate.build_multiformat_conformance_plan import (
    ConformancePlanError,
    validate_conformance_plan,
)
from evaluate.multiformat_candidate_artifacts import write_canonical_json
from evaluate.multiformat_conformance_docx import (
    DocxConformanceError,
    docx_case_bytes,
)
from evaluate.multiformat_corpus_items import object_list
from evaluate.multiformat_corpus_sources import validate_source
from evaluate.multiformat_corpus_types import CorpusError, DocumentFormat
from evaluate.multiformat_evaluator_files import EVALUATOR_FILES
from evaluate.multiformat_schema import (
    JsonValue,
    integer_value,
    object_value,
    sha256_file,
    string_value,
)
from evaluate.multiformat_strict_json import StrictJsonError, read_strict_object

GENERATOR_FILES: Final[tuple[str, ...]] = (
    "evaluate/generate_multiformat_docx_conformance.py",
    "evaluate/multiformat_conformance_docx.py",
    "evaluate/multiformat_conformance_docx_parts.py",
)


def generate_docx_conformance(contract: Path, plan: Path, output_dir: Path) -> Path:
    if output_dir.exists():
        raise DocxConformanceError("DOCX conformance output already exists")
    try:
        validate_conformance_plan(contract, plan)
        plan_values = read_strict_object(plan)
        docx_values = object_value(object_value(plan_values, "formats"), "docx")
        cases = object_list(docx_values, "cases", "docx.conformance.cases")
        if len(cases) != 100:
            raise DocxConformanceError("DOCX conformance requires 100 cases")
        output_dir.mkdir(parents=True)
        source_root = output_dir / "sources" / "docx"
        source_root.mkdir(parents=True)
        files = _materialize(cases, source_root, output_dir)
        project_root = Path(__file__).resolve().parents[1]
        manifest = output_dir / "generation-manifest.json"
        write_canonical_json(
            manifest,
            {
                "schema_version": 1,
                "status": "FROZEN",
                "format": "docx",
                "contract_sha256": sha256_file(contract),
                "plan_sha256": sha256_file(plan),
                "generator_sha256": _source_set_hash(project_root, GENERATOR_FILES),
                "evaluator_sha256": _source_set_hash(project_root, EVALUATOR_FILES),
                "inventory_sha256": _canonical_hash(files),
                "snapshot_sha256": _snapshot_hash(files),
                "evaluation": "Portable LibreOffice rendering with Poppler inspection",
                "files": files,
            },
        )
        _validate_output_set(output_dir, files)
        return manifest
    except DocxConformanceError:
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
        raise DocxConformanceError("DOCX conformance generation failed") from error


def _materialize(
    cases: list[dict[str, JsonValue]],
    source_root: Path,
    evidence_root: Path,
) -> list[dict[str, JsonValue]]:
    files: list[dict[str, JsonValue]] = []
    for case in cases:
        case_id = string_value(case, "id")
        source = source_root / f"{case_id}.docx"
        source.write_bytes(docx_case_bytes(case))
        source.chmod(0o444)
        digest = sha256_file(source)
        relative_path = source.relative_to(evidence_root).as_posix()
        validate_source(
            {"id": case_id, "path": relative_path, "sha256": digest},
            evidence_root,
            DocumentFormat.DOCX,
            require_valid_format=True,
        )
        files.append(
            {
                "id": case_id,
                "ordinal": integer_value(case, "ordinal"),
                "primary_stratum": string_value(case, "primary_stratum"),
                "feature_seed": string_value(case, "feature_seed"),
                "path": relative_path,
                "sha256": digest,
                "unit_count": 1,
            }
        )
    return files


def _source_set_hash(root: Path, paths: tuple[str, ...]) -> str:
    bindings = [{"path": path, "sha256": sha256_file(root / path)} for path in paths]
    return _canonical_hash(bindings)


def _canonical_hash(value: JsonValue) -> str:
    encoded = json.dumps(
        value, ensure_ascii=True, separators=(",", ":"), sort_keys=True
    )
    return hashlib.sha256(encoded.encode()).hexdigest()


def _snapshot_hash(files: list[dict[str, JsonValue]]) -> str:
    identity = [
        {"path": string_value(item, "path"), "sha256": string_value(item, "sha256")}
        for item in files
    ]
    return _canonical_hash(identity)


def _validate_output_set(root: Path, files: list[dict[str, JsonValue]]) -> None:
    expected = {
        root / "generation-manifest.json",
        *(root / string_value(item, "path") for item in files),
    }
    actual = {path for path in root.rglob("*") if path.is_file() or path.is_symlink()}
    if actual != expected:
        raise DocxConformanceError("DOCX conformance file set differs")


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Materialize the frozen portable DOCX conformance snapshot.",
    )
    parser.add_argument("--contract", type=Path, required=True)
    parser.add_argument("--plan", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    arguments = parser.parse_args()
    try:
        generate_docx_conformance(
            arguments.contract, arguments.plan, arguments.output_dir
        )
    except DocxConformanceError as error:
        parser.error(str(error))


if __name__ == "__main__":
    main()
