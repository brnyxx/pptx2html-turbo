from __future__ import annotations

import argparse
import hashlib
from pathlib import Path

from evaluate.multiformat_candidate_artifacts import write_canonical_json
from evaluate.multiformat_corpus_items import require_keys
from evaluate.multiformat_schema import (
    JsonValue,
    integer_value,
    object_value,
    sha256_file,
    string_list,
)
from evaluate.multiformat_strict_json import read_strict_object

LEGACY_PAIRS = {
    "doc": "docx",
    "xls": "xlsx",
    "ppt": "pptx",
}


class ConformancePlanError(Exception):
    pass


def build_conformance_plan(contract: Path, output: Path) -> Path:
    if output.exists():
        raise ConformancePlanError("conformance plan output already exists")
    try:
        contract_values = read_strict_object(contract)
        value = _plan_value(contract_values, sha256_file(contract))
        write_canonical_json(output, value)
        validate_conformance_plan(contract, output)
        return output
    except ConformancePlanError:
        raise
    except (OSError, TypeError, ValueError) as error:
        raise ConformancePlanError("conformance plan build failed") from error


def validate_conformance_plan(contract: Path, plan: Path) -> None:
    try:
        contract_values = read_strict_object(contract)
        expected = _plan_value(contract_values, sha256_file(contract))
        actual = read_strict_object(plan)
        if actual != expected:
            raise ConformancePlanError("conformance plan differs from contract")
    except ConformancePlanError:
        raise
    except (OSError, TypeError, ValueError) as error:
        raise ConformancePlanError("conformance plan validation failed") from error


def _plan_value(
    contract: dict[str, JsonValue],
    contract_sha256: str,
) -> dict[str, JsonValue]:
    require_keys(
        contract,
        {
            "schema_version",
            "required_formats",
            "corpus",
            "thresholds",
            "strata",
            "stratum_quotas",
            "legacy_paired_stratum_quotas",
            "security_case_outcomes",
            "metric_parameters",
        },
        "conformance.plan.contract",
    )
    required_formats = string_list(contract, "required_formats")
    corpus = object_value(contract, "corpus")
    expected_count = integer_value(corpus, "conformance_units")
    if expected_count != 100:
        raise ConformancePlanError("conformance plan requires 100 units")
    quotas = object_value(contract, "stratum_quotas")
    paired_quotas = object_value(contract, "legacy_paired_stratum_quotas")
    formats: dict[str, JsonValue] = {}
    modern_ids: dict[str, dict[str, list[str]]] = {}
    for document_format in required_formats:
        if document_format in LEGACY_PAIRS:
            continue
        cases, by_stratum = _modern_cases(
            document_format,
            object_value(quotas, document_format),
            expected_count,
        )
        formats[document_format] = {"expected_count": expected_count, "cases": cases}
        modern_ids[document_format] = by_stratum
    for document_format, paired_format in LEGACY_PAIRS.items():
        cases = _legacy_cases(
            document_format,
            paired_format,
            object_value(quotas, document_format),
            object_value(paired_quotas, document_format),
            modern_ids[paired_format],
            expected_count,
        )
        formats[document_format] = {"expected_count": expected_count, "cases": cases}
    if set(formats) != set(required_formats):
        raise ConformancePlanError("conformance plan format set differs")
    return {
        "schema_version": 1,
        "status": "PLANNED",
        "contract_sha256": contract_sha256,
        "formats": formats,
    }


def _modern_cases(
    document_format: str,
    quotas: dict[str, JsonValue],
    expected_count: int,
) -> tuple[list[dict[str, JsonValue]], dict[str, list[str]]]:
    cases: list[dict[str, JsonValue]] = []
    by_stratum: dict[str, list[str]] = {}
    ordinal = 0
    for stratum, count_value in quotas.items():
        if not isinstance(count_value, int) or isinstance(count_value, bool):
            raise ConformancePlanError("conformance stratum quota is invalid")
        by_stratum[stratum] = []
        for _ in range(count_value):
            ordinal += 1
            case_id = f"{document_format}-conformance-{ordinal:03d}"
            by_stratum[stratum].append(case_id)
            cases.append(
                _case_value(
                    case_id,
                    ordinal,
                    stratum,
                    None,
                    "generated-modern",
                    None,
                )
            )
    if ordinal != expected_count:
        raise ConformancePlanError("conformance stratum quota differs")
    return cases, by_stratum


def _legacy_cases(
    document_format: str,
    paired_format: str,
    quotas: dict[str, JsonValue],
    paired_quotas: dict[str, JsonValue],
    modern_ids: dict[str, list[str]],
    expected_count: int,
) -> list[dict[str, JsonValue]]:
    if quotas != {"paired-legacy": 60, "binary-specific": 40}:
        raise ConformancePlanError("legacy conformance quota differs")
    cases: list[dict[str, JsonValue]] = []
    ordinal = 0
    for stratum, count_value in paired_quotas.items():
        if (
            not isinstance(count_value, int)
            or isinstance(count_value, bool)
            or count_value > len(modern_ids.get(stratum, []))
        ):
            raise ConformancePlanError("legacy paired quota is invalid")
        for index in range(count_value):
            ordinal += 1
            case_id = f"{document_format}-conformance-{ordinal:03d}"
            cases.append(
                _case_value(
                    case_id,
                    ordinal,
                    "paired-legacy",
                    stratum,
                    "paired-legacy",
                    modern_ids[stratum][index],
                )
            )
    while ordinal < expected_count:
        ordinal += 1
        case_id = f"{document_format}-conformance-{ordinal:03d}"
        cases.append(
            _case_value(
                case_id,
                ordinal,
                "binary-specific",
                None,
                "public-binary",
                None,
            )
        )
    return cases


def _case_value(
    case_id: str,
    ordinal: int,
    primary_stratum: str,
    paired_stratum: str | None,
    source_kind: str,
    paired_case_id: str | None,
) -> dict[str, JsonValue]:
    return {
        "id": case_id,
        "ordinal": ordinal,
        "primary_stratum": primary_stratum,
        "paired_stratum": paired_stratum,
        "source_kind": source_kind,
        "paired_case_id": paired_case_id,
        "feature_seed": hashlib.sha256(case_id.encode()).hexdigest(),
    }


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Build the exact seven-format conformance case plan.",
    )
    parser.add_argument("--contract", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    arguments = parser.parse_args()
    try:
        build_conformance_plan(arguments.contract, arguments.output)
    except ConformancePlanError as error:
        parser.error(str(error))


if __name__ == "__main__":
    main()
