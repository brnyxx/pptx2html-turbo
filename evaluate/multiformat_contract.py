from __future__ import annotations

import json
from pathlib import Path

from evaluate.multiformat_checks import (
    check_hard_gates,
    check_strata,
    check_track,
    minimum,
)
from evaluate.multiformat_corpus import (
    CorpusError,
    CorpusStatus,
    validate_corpus_manifest,
)
from evaluate.multiformat_evidence import bound_evidence_path, oracle_lock_ready
from evaluate.multiformat_gate_types import (
    FormatGateResult,
    GateStatus,
    GateSummary,
)
from evaluate.multiformat_report_validation import validate_generated_report
from evaluate.multiformat_schema import (
    JsonValue,
    integer_value,
    object_value,
    sha256_file,
    string_list,
)
from evaluate.multiformat_strict_json import read_strict_object


def evaluate_reports(
    contract_path: Path,
    reports_dir: Path,
    oracle_lock_path: Path,
    evidence_root: Path | None = None,
) -> GateSummary:
    contract = read_strict_object(contract_path)
    required_formats = string_list(contract, "required_formats")
    if not oracle_lock_ready(oracle_lock_path):
        results = tuple(
            FormatGateResult(
                format=document_format,
                status=GateStatus.INCOMPLETE,
                reasons=("oracle_lock",),
            )
            for document_format in required_formats
        )
        return GateSummary(status=GateStatus.INCOMPLETE, formats=results)

    contract_hash = sha256_file(contract_path)
    lock_hash = sha256_file(oracle_lock_path)
    resolved_evidence_root = evidence_root or reports_dir.parent
    results = tuple(
        _evaluate_format(
            document_format=document_format,
            report_path=reports_dir / f"{document_format}.json",
            contract_path=contract_path,
            contract=contract,
            contract_hash=contract_hash,
            lock_hash=lock_hash,
            evidence_root=resolved_evidence_root,
        )
        for document_format in required_formats
    )
    if any(result.status is GateStatus.FAIL for result in results):
        status = GateStatus.FAIL
    elif any(result.status is GateStatus.INCOMPLETE for result in results):
        status = GateStatus.INCOMPLETE
    else:
        status = GateStatus.PASS
    return GateSummary(status=status, formats=results)


def _evaluate_format(
    *,
    document_format: str,
    report_path: Path,
    contract_path: Path,
    contract: dict[str, JsonValue],
    contract_hash: str,
    lock_hash: str,
    evidence_root: Path,
) -> FormatGateResult:
    if not report_path.is_file():
        return FormatGateResult(document_format, GateStatus.INCOMPLETE, ("report",))
    try:
        report = read_strict_object(report_path)
        report_status = report.get("status")
        if report_status == "INCOMPLETE":
            missing = tuple(string_list(report, "missing"))
            return FormatGateResult(
                document_format,
                GateStatus.INCOMPLETE,
                missing or ("report",),
            )
        if report_status != "READY":
            return FormatGateResult(
                document_format,
                GateStatus.FAIL,
                ("report_schema",),
            )
        reasons = _report_failures(
            document_format,
            report,
            contract_path,
            contract,
            contract_hash,
            lock_hash,
            evidence_root,
        )
    except (OSError, UnicodeError, json.JSONDecodeError, ValueError, TypeError):
        return FormatGateResult(document_format, GateStatus.FAIL, ("report_schema",))
    status = GateStatus.FAIL if reasons else GateStatus.PASS
    return FormatGateResult(document_format, status, tuple(reasons))


def _report_failures(
    document_format: str,
    report: dict[str, JsonValue],
    contract_path: Path,
    contract: dict[str, JsonValue],
    contract_hash: str,
    lock_hash: str,
    evidence_root: Path,
) -> list[str]:
    failures: list[str] = []
    _require_equal(report, "schema_version", 2, "schema_version", failures)
    _require_equal(report, "format", document_format, "format", failures)
    _require_equal(
        report, "contract_sha256", contract_hash, "contract_sha256", failures
    )
    _require_equal(
        report, "oracle_lock_sha256", lock_hash, "oracle_lock_sha256", failures
    )
    evidence_paths = {
        field: bound_evidence_path(report, field, evidence_root, failures)
        for field in ["evaluator", "corpus_manifest", "metrics_evidence"]
    }
    corpus_path = evidence_paths["corpus_manifest"]
    corpus_ready = False
    if corpus_path is not None:
        try:
            validation = validate_corpus_manifest(contract_path, corpus_path)
            if (
                validation.status is not CorpusStatus.READY
                or validation.document_format.value != document_format
            ):
                failures.append("corpus_manifest")
            else:
                corpus_ready = True
        except CorpusError:
            failures.append("corpus_manifest")
    evaluator_path = evidence_paths["evaluator"]
    metrics_path = evidence_paths["metrics_evidence"]
    if (
        corpus_ready
        and corpus_path is not None
        and evaluator_path is not None
        and metrics_path is not None
    ):
        failures.extend(
            validate_generated_report(
                report,
                contract_path,
                contract,
                contract_hash,
                lock_hash,
                evidence_root,
                evaluator_path,
                corpus_path,
                metrics_path,
            )
        )
    corpus = object_value(contract, "corpus")
    thresholds = object_value(contract, "thresholds")
    conformance = object_value(report, "conformance")
    blind = object_value(report, "blind")
    _require_equal(
        conformance,
        "unit_count",
        integer_value(corpus, "conformance_units"),
        "conformance.unit_count",
        failures,
    )
    _require_equal(
        blind,
        "file_count",
        integer_value(corpus, "blind_files"),
        "blind.file_count",
        failures,
    )
    _require_equal(
        blind,
        "accepted_files",
        integer_value(corpus, "blind_files"),
        "blind.accepted_files",
        failures,
    )
    _require_equal(blind, "critical_defects", 0, "blind.critical_defects", failures)
    check_track("conformance", conformance, thresholds, failures)
    check_track("blind", blind, thresholds, failures)
    minimum(
        conformance,
        "minimum_unit_score",
        thresholds,
        "minimum_unit_score",
        "conformance.minimum_unit_score",
        failures,
    )
    minimum(
        blind,
        "minimum_file_score",
        thresholds,
        "minimum_blind_file_score",
        "blind.minimum_file_score",
        failures,
    )
    check_strata(document_format, conformance, contract, thresholds, failures)
    check_hard_gates(report, corpus, failures)
    return failures


def _require_equal(
    values: dict[str, JsonValue],
    field: str,
    expected: JsonValue,
    reason: str,
    failures: list[str],
) -> None:
    if values.get(field) != expected:
        failures.append(reason)
