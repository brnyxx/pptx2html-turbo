from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import tempfile
from collections import Counter
from pathlib import Path

from evaluate.build_multiformat_conformance_plan import (
    ConformancePlanError,
    validate_conformance_plan,
)
from evaluate.multiformat_candidate_artifacts import write_canonical_json
from evaluate.multiformat_conformance_xlsx import (
    XlsxConformanceError,
    inspect_xlsx_package,
    xlsx_case_package,
)
from evaluate.multiformat_corpus_items import object_list
from evaluate.multiformat_corpus_types import CorpusError
from evaluate.multiformat_schema import (
    JsonValue,
    integer_value,
    object_value,
    sha256_file,
    sha256_value,
    string_value,
)
from evaluate.multiformat_strict_json import StrictJsonError, read_strict_object

_EXPECTED_QUOTAS = {
    "values-formulas": 25,
    "styles-conditional-formats": 20,
    "print-layout": 20,
    "charts-images-shapes": 15,
    "international-formats": 10,
    "mixed-stress": 10,
}
_GENERATOR_SOURCES = (
    "evaluate/generate_multiformat_xlsx_conformance.py",
    "evaluate/multiformat_conformance_xlsx.py",
    "evaluate/multiformat_xlsx_features.py",
    "evaluate/multiformat_xlsx_package_validation.py",
    "evaluate/multiformat_xlsx_parts.py",
)
_EVALUATOR_SOURCES = (
    "evaluate/build_multiformat_conformance_plan.py",
    "evaluate/multiformat_conformance_xlsx.py",
    "evaluate/multiformat_xlsx_package_validation.py",
)


def generate_xlsx_conformance(
    contract: Path,
    plan: Path,
    output_dir: Path,
) -> Path:
    if output_dir.exists():
        raise XlsxConformanceError("XLSX conformance output already exists")
    published = False
    try:
        validate_conformance_plan(contract, plan)
        cases = _planned_cases(plan)
        output_dir.parent.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(
            prefix=".xlsx-conformance-",
            dir=output_dir.parent,
        ) as temp_dir:
            staging = Path(temp_dir) / "snapshot"
            source_root = staging / "sources" / "xlsx"
            source_root.mkdir(parents=True)
            files = _materialize_sources(cases, source_root, staging)
            manifest_value = _manifest_value(contract, plan, files)
            manifest = staging / "generation-manifest.json"
            write_canonical_json(manifest, manifest_value)
            _validate_staging(staging, files)
            staging.rename(output_dir)
            published = True
        _make_immutable(output_dir)
        return output_dir / "generation-manifest.json"
    except XlsxConformanceError:
        if published:
            _remove_owned_output(output_dir)
        raise
    except (
        ConformancePlanError,
        CorpusError,
        OSError,
        StrictJsonError,
        TypeError,
        ValueError,
    ) as error:
        if published:
            _remove_owned_output(output_dir)
        raise XlsxConformanceError("XLSX conformance generation failed") from error


def _planned_cases(plan: Path) -> list[dict[str, JsonValue]]:
    plan_values = read_strict_object(plan)
    format_values = object_value(object_value(plan_values, "formats"), "xlsx")
    cases = object_list(format_values, "cases", "xlsx.conformance.cases")
    if integer_value(format_values, "expected_count") != 100 or len(cases) != 100:
        raise XlsxConformanceError("XLSX conformance requires 100 cases")
    quotas = Counter(string_value(case, "primary_stratum") for case in cases)
    if quotas != _EXPECTED_QUOTAS:
        raise XlsxConformanceError("XLSX conformance stratum quota differs")
    if [integer_value(case, "ordinal") for case in cases] != list(range(1, 101)):
        raise XlsxConformanceError("XLSX conformance ordinals differ")
    return cases


def _materialize_sources(
    cases: list[dict[str, JsonValue]],
    source_root: Path,
    evidence_root: Path,
) -> list[dict[str, JsonValue]]:
    files: list[dict[str, JsonValue]] = []
    for case in cases:
        case_id = string_value(case, "id")
        path = source_root / f"{case_id}.xlsx"
        package = xlsx_case_package(case)
        inspection = inspect_xlsx_package(package)
        path.write_bytes(package)
        files.append(
            {
                "id": case_id,
                "ordinal": integer_value(case, "ordinal"),
                "feature_seed": sha256_value(case, "feature_seed"),
                "primary_stratum": string_value(case, "primary_stratum"),
                "path": path.relative_to(evidence_root).as_posix(),
                "sha256": sha256_file(path),
                "unit_count": 1,
                "package_inventory": list(inspection.inventory),
                "admissions": sorted(inspection.admissions),
            },
        )
    return files


def _manifest_value(
    contract: Path,
    plan: Path,
    files: list[dict[str, JsonValue]],
) -> dict[str, JsonValue]:
    project_root = Path(__file__).resolve().parents[1]
    contract_hash = sha256_file(contract)
    plan_hash = sha256_file(plan)
    source_hashes = {
        string_value(item, "id"): sha256_value(item, "sha256") for item in files
    }
    identity_value: dict[str, JsonValue] = {
        "contract_sha256": contract_hash,
        "plan_sha256": plan_hash,
        "files": files,
        "source_sha256": source_hashes,
    }
    identity_hash = hashlib.sha256(
        json.dumps(
            identity_value,
            ensure_ascii=True,
            separators=(",", ":"),
            sort_keys=True,
        ).encode(),
    ).hexdigest()
    return {
        "schema_version": 1,
        "status": "GENERATED",
        "format": "xlsx",
        "contract_sha256": contract_hash,
        "plan_sha256": plan_hash,
        "frozen_snapshot": {
            "identity_sha256": identity_hash,
            "normative": True,
        },
        "generator_reproducibility": {
            "normative": False,
            "source_bindings": _source_bindings(project_root, _GENERATOR_SOURCES),
        },
        "evaluator_source_bindings": _source_bindings(
            project_root,
            _EVALUATOR_SOURCES,
        ),
        "portable_evaluation": {
            "office_engine": "LibreOffice",
            "pdf_engine": "Poppler",
        },
        "source_sha256": source_hashes,
        "files": files,
    }


def _source_bindings(
    project_root: Path,
    relative_paths: tuple[str, ...],
) -> list[dict[str, JsonValue]]:
    return [
        {"path": relative_path, "sha256": sha256_file(project_root / relative_path)}
        for relative_path in relative_paths
    ]


def _validate_staging(
    root: Path,
    files: list[dict[str, JsonValue]],
) -> None:
    expected = {
        root / "generation-manifest.json",
        *(root / string_value(item, "path") for item in files),
    }
    actual = {path for path in root.rglob("*") if path.is_file() or path.is_symlink()}
    if actual != expected:
        raise XlsxConformanceError("XLSX conformance file set differs")
    for item in files:
        source = root / string_value(item, "path")
        if sha256_file(source) != sha256_value(item, "sha256"):
            raise XlsxConformanceError("XLSX conformance source hash differs")
        inspect_xlsx_package(source.read_bytes())


def _make_immutable(root: Path) -> None:
    for path in sorted(root.rglob("*"), reverse=True):
        path.chmod(0o555 if path.is_dir() else 0o444)
    root.chmod(0o555)


def _remove_owned_output(root: Path) -> None:
    if not root.exists():
        return
    for path in root.rglob("*"):
        if path.is_dir():
            path.chmod(0o755)
        else:
            path.chmod(0o644)
    root.chmod(0o755)
    shutil.rmtree(root)


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Materialize the deterministic XLSX conformance snapshot.",
    )
    parser.add_argument("--contract", type=Path, required=True)
    parser.add_argument("--plan", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    arguments = parser.parse_args()
    try:
        generate_xlsx_conformance(
            arguments.contract,
            arguments.plan,
            arguments.output_dir,
        )
    except XlsxConformanceError as error:
        parser.error(str(error))


if __name__ == "__main__":
    main()
