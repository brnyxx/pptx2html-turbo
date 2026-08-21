from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import stat
from pathlib import Path
from typing import Final

from evaluate.build_multiformat_conformance_plan import (
    ConformancePlanError,
    validate_conformance_plan,
)
from evaluate.multiformat_candidate_artifacts import write_canonical_json
from evaluate.multiformat_conformance_pptx import (
    PptxConformanceError,
    package_inventory,
    pptx_case_bytes,
)
from evaluate.multiformat_corpus_items import object_list
from evaluate.multiformat_evaluator_files import EVALUATOR_FILES
from evaluate.multiformat_schema import (
    JsonValue,
    integer_value,
    object_value,
    sha256_file,
    string_value,
)
from evaluate.multiformat_strict_json import StrictJsonError, read_strict_object

GENERATOR_FILES: Final = (
    "evaluate/generate_multiformat_pptx_conformance.py",
    "evaluate/multiformat_conformance_pptx.py",
    "evaluate/completion_deck_package.py",
    "evaluate/completion_deck_common.py",
    "evaluate/completion_deck_charts.py",
    "evaluate/completion_deck_tables.py",
    "evaluate/completion_deck_specs.py",
)


def generate_pptx_conformance(
    contract: Path,
    plan: Path,
    output_dir: Path,
) -> Path:
    if output_dir.exists():
        raise PptxConformanceError("PPTX conformance output already exists")
    try:
        validate_conformance_plan(contract, plan)
        project_root = contract.resolve(strict=True).parents[2]
        cases = object_list(
            object_value(
                object_value(read_strict_object(plan), "formats"),
                "pptx",
            ),
            "cases",
            "pptx.conformance.cases",
        )
        if len(cases) != 100:
            raise PptxConformanceError("PPTX conformance requires 100 cases")
        source_root = output_dir / "sources" / "pptx"
        source_root.mkdir(parents=True)
        files = _materialize_cases(cases, source_root, output_dir)
        generator_binding = _file_binding(project_root, GENERATOR_FILES)
        evaluator_binding = _file_binding(project_root, EVALUATOR_FILES)
        contract_sha256 = sha256_file(contract)
        plan_sha256 = sha256_file(plan)
        snapshot_sha256 = _canonical_sha256(
            {
                "contract_sha256": contract_sha256,
                "plan_sha256": plan_sha256,
                "files": files,
            }
        )
        reproducibility_sha256 = _canonical_sha256(
            {
                "generator": generator_binding,
                "evaluator": evaluator_binding,
            }
        )
        manifest = output_dir / "generation-manifest.json"
        write_canonical_json(
            manifest,
            {
                "schema_version": 1,
                "status": "GENERATED",
                "format": "pptx",
                "contract_sha256": contract_sha256,
                "plan_sha256": plan_sha256,
                "snapshot_identity": {
                    "kind": "frozen-sources",
                    "sha256": snapshot_sha256,
                },
                "generator_reproducibility": {
                    "kind": "deterministic-generator",
                    "sha256": reproducibility_sha256,
                    "generator_files": generator_binding,
                    "evaluator_files": evaluator_binding,
                },
                "files": files,
            },
        )
        _validate_output_set(output_dir, files)
        return manifest
    except PptxConformanceError:
        if output_dir.exists():
            shutil.rmtree(output_dir)
        raise
    except (
        ConformancePlanError,
        OSError,
        StrictJsonError,
        TypeError,
        ValueError,
    ) as error:
        if output_dir.exists():
            shutil.rmtree(output_dir)
        raise PptxConformanceError("PPTX conformance generation failed") from error


def _materialize_cases(
    cases: list[dict[str, JsonValue]],
    source_root: Path,
    output_dir: Path,
) -> list[dict[str, JsonValue]]:
    files: list[dict[str, JsonValue]] = []
    for case in cases:
        case_id = string_value(case, "id")
        source = source_root / f"{case_id}.pptx"
        value = pptx_case_bytes(case)
        source.write_bytes(value)
        source.chmod(stat.S_IRUSR | stat.S_IRGRP | stat.S_IROTH)
        files.append(
            {
                "id": case_id,
                "ordinal": integer_value(case, "ordinal"),
                "feature_seed": string_value(case, "feature_seed"),
                "primary_stratum": string_value(case, "primary_stratum"),
                "path": source.relative_to(output_dir).as_posix(),
                "sha256": hashlib.sha256(value).hexdigest(),
                "unit_count": 1,
                "package_inventory": package_inventory(value),
            }
        )
    return files


def _file_binding(
    project_root: Path,
    relative_paths: tuple[str, ...],
) -> list[dict[str, JsonValue]]:
    return [
        {
            "path": relative_path,
            "sha256": sha256_file(project_root / relative_path),
        }
        for relative_path in relative_paths
    ]


def _canonical_sha256(value: JsonValue) -> str:
    encoded = json.dumps(
        value,
        ensure_ascii=True,
        separators=(",", ":"),
        sort_keys=True,
    ).encode()
    return hashlib.sha256(encoded).hexdigest()


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
        raise PptxConformanceError("PPTX conformance file set differs")


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Materialize the immutable PPTX conformance snapshot.",
    )
    parser.add_argument("--contract", type=Path, required=True)
    parser.add_argument("--plan", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    arguments = parser.parse_args()
    try:
        generate_pptx_conformance(
            arguments.contract,
            arguments.plan,
            arguments.output_dir,
        )
    except PptxConformanceError as error:
        parser.error(str(error))


if __name__ == "__main__":
    main()
