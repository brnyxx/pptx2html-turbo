from __future__ import annotations

import json
from dataclasses import dataclass
from enum import StrEnum
from pathlib import Path

from evaluate.multiformat_checks import (
    check_hard_gates,
    check_strata,
    check_track,
    minimum,
)
from evaluate.multiformat_schema import (
    JsonValue,
    integer_value,
    object_value,
    read_object,
    sha256_file,
    sha256_value,
    string_list,
    string_value,
)


class GateStatus(StrEnum):
    PASS = "PASS"
    FAIL = "FAIL"
    INCOMPLETE = "INCOMPLETE"


@dataclass(frozen=True, slots=True)
class FormatGateResult:
    format: str
    status: GateStatus
    reasons: tuple[str, ...]


@dataclass(frozen=True, slots=True)
class GateSummary:
    status: GateStatus
    formats: tuple[FormatGateResult, ...]

    def to_json_value(self) -> dict[str, JsonValue]:
        return {
            "status": self.status.value,
            "formats": [
                {
                    "format": result.format,
                    "status": result.status.value,
                    "reasons": list(result.reasons),
                }
                for result in self.formats
            ],
        }


def evaluate_reports(
    contract_path: Path,
    reports_dir: Path,
    oracle_lock_path: Path,
    evidence_root: Path | None = None,
) -> GateSummary:
    contract = read_object(contract_path)
    required_formats = string_list(contract, "required_formats")
    if not _oracle_lock_ready(oracle_lock_path):
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
    contract: dict[str, JsonValue],
    contract_hash: str,
    lock_hash: str,
    evidence_root: Path,
) -> FormatGateResult:
    if not report_path.is_file():
        return FormatGateResult(document_format, GateStatus.INCOMPLETE, ("report",))
    try:
        report = read_object(report_path)
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
    contract: dict[str, JsonValue],
    contract_hash: str,
    lock_hash: str,
    evidence_root: Path,
) -> list[str]:
    failures: list[str] = []
    _require_equal(report, "schema_version", 1, "schema_version", failures)
    _require_equal(report, "format", document_format, "format", failures)
    _require_equal(
        report, "contract_sha256", contract_hash, "contract_sha256", failures
    )
    _require_equal(
        report, "oracle_lock_sha256", lock_hash, "oracle_lock_sha256", failures
    )
    for field in ["evaluator", "corpus_manifest", "metrics_evidence"]:
        _check_evidence_binding(report, field, evidence_root, failures)
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


def _check_evidence_binding(
    report: dict[str, JsonValue],
    field: str,
    evidence_root: Path,
    failures: list[str],
) -> None:
    try:
        binding = object_value(report, field)
        relative_path = string_value(binding, "path")
        expected_hash = sha256_value(binding, "sha256")
        evidence_path = _resolve_evidence_path(evidence_root, relative_path)
        if sha256_file(evidence_path) != expected_hash:
            failures.append(field)
    except (OSError, TypeError, ValueError):
        failures.append(field)


def _resolve_evidence_path(root: Path, relative_path: str) -> Path:
    relative = Path(relative_path)
    if (
        relative.is_absolute()
        or "\\" in relative_path
        or relative_path != relative.as_posix()
        or any(part in {"", ".", ".."} for part in relative.parts)
    ):
        raise ValueError("evidence path must be normalized and relative")
    resolved_root = root.resolve(strict=True)
    candidate = resolved_root
    for part in relative.parts:
        candidate /= part
        if candidate.is_symlink():
            raise ValueError("evidence path cannot contain symlinks")
    candidate = candidate.resolve(strict=True)
    if not candidate.is_relative_to(resolved_root) or not candidate.is_file():
        raise ValueError("evidence path escapes the evidence root")
    return candidate


def _require_equal(
    values: dict[str, JsonValue],
    field: str,
    expected: JsonValue,
    reason: str,
    failures: list[str],
) -> None:
    if values.get(field) != expected:
        failures.append(reason)


def _oracle_lock_ready(path: Path) -> bool:
    if not path.is_file():
        return False
    try:
        lock = read_object(path)
        if lock.get("schema_version") != 1 or lock.get("status") != "locked":
            return False
        office = object_value(lock, "office")
        pdf = object_value(lock, "pdf")
        browser = object_value(lock, "browser")
        for field in ["os", "word", "excel", "powerpoint"]:
            string_value(office, field)
        for field in ["primary", "secondary"]:
            string_value(pdf, field)
        string_value(browser, "chromium")
        font_hash = string_value(lock, "font_bundle_sha256")
        return len(font_hash) == 64 and all(
            character in "0123456789abcdef" for character in font_hash
        )
    except (OSError, UnicodeError, json.JSONDecodeError, ValueError, TypeError):
        return False
