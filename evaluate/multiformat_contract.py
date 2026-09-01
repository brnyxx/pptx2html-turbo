from __future__ import annotations

import hashlib
import json
from pathlib import Path

from evaluate.jcs import canonicalize
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
from evaluate.multiformat_evidence import bound_evidence_path
from evaluate.multiformat_gate_types import (
    FormatGateResult,
    GateStatus,
    GateSummary,
    OracleLockInput,
    OracleLockInputError,
    ResolvedOracleLock,
)
from evaluate.multiformat_report_validation import (
    validate_generated_report,
    require_equal,
    validate_oracle_scope,
)
from evaluate.multiformat_schema import (
    JsonValue,
    integer_value,
    object_value,
    sha256_file,
    string_list,
)
from evaluate.multiformat_strict_json import read_strict_object


def contract_digest(contract_path: Path) -> str:
    """Return the RFC 8785 identity of every contract field."""
    return hashlib.sha256(canonicalize(read_strict_object(contract_path))).hexdigest()


def evaluate_reports(
    contract_path: Path,
    reports_dir: Path,
    oracle_locks: Path | OracleLockInput,
    evidence_root: Path | None = None,
) -> GateSummary:
    contract = read_strict_object(contract_path)
    required_formats = string_list(contract, "required_formats")
    resolved_evidence_root = evidence_root or reports_dir.parent
    inputs = (
        OracleLockInput(shared=oracle_locks)
        if isinstance(oracle_locks, Path)
        else oracle_locks
    )
    try:
        locks = inputs.resolve(required_formats, resolved_evidence_root)
    except OracleLockInputError as error:
        results = tuple(
            FormatGateResult(document_format, error.status, (error.reason,))
            for document_format in required_formats
        )
        return GateSummary(status=error.status, formats=results)

    contract_hash = sha256_file(contract_path)
    results = tuple(
        _evaluate_format(
            document_format=document_format,
            report_path=reports_dir / f"{document_format}.json",
            contract_path=contract_path,
            contract=contract,
            contract_hash=contract_hash,
            oracle_lock=locks[document_format],
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
    oracle_lock: ResolvedOracleLock,
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
            oracle_lock,
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
    oracle_lock: ResolvedOracleLock,
    evidence_root: Path,
) -> list[str]:
    failures: list[str] = []
    require_equal(report, "schema_version", 2, "schema_version", failures)
    require_equal(report, "format", document_format, "format", failures)
    require_equal(report, "contract_sha256", contract_hash, "contract_sha256", failures)
    require_equal(
        report, "oracle_lock_sha256", oracle_lock.sha256, "oracle_lock_sha256", failures
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
                failures.extend(
                    validate_oracle_scope(
                        oracle_lock.portable, document_format, corpus_path
                    )
                )
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
                oracle_lock.sha256,
                evidence_root,
                evaluator_path,
                corpus_path,
                metrics_path,
                oracle_lock.path,
            )
        )
    corpus = object_value(contract, "corpus")
    thresholds = object_value(contract, "thresholds")
    conformance = object_value(report, "conformance")
    blind = object_value(report, "blind")
    require_equal(
        conformance,
        "unit_count",
        integer_value(corpus, "conformance_units"),
        "conformance.unit_count",
        failures,
    )
    require_equal(
        blind,
        "file_count",
        integer_value(corpus, "blind_files"),
        "blind.file_count",
        failures,
    )
    require_equal(
        blind,
        "accepted_files",
        integer_value(corpus, "blind_files"),
        "blind.accepted_files",
        failures,
    )
    require_equal(blind, "critical_defects", 0, "blind.critical_defects", failures)
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
