from __future__ import annotations

import hashlib
import json
import unittest
from pathlib import Path

from evaluate.multiformat_evaluator_files import EVALUATOR_FILES
from evaluate.multiformat_revision import current_project_revision
from evaluate.multiformat_schema import (
    JsonValue,
    object_value,
    read_object,
    string_list,
)
from evaluate.tests.multiformat_corpus_fixture import (
    PAIRED_FORMATS,
    write_corpus,
)
from evaluate.tests.multiformat_candidate_gate_lock_fixture import (
    write_gate_oracle_lock,
)
from evaluate.tests.multiformat_metrics_fixture import (
    patched_reviewer_registry,
    write_metrics,
)

PROJECT_ROOT = Path(__file__).resolve().parents[2]
CONTRACT_PATH = PROJECT_ROOT / "evaluate" / "multiformat" / "contract.v1.json"


class MultiFormatGateFixture(unittest.TestCase):
    def setUp(self) -> None:
        # Fixture metrics carry decisions signed by the deterministic test-only
        # reviewers, so every consumer of this fixture resolves reviewer trust
        # through the matching test registry instead of the tracked one.
        super().setUp()
        self.enterContext(patched_reviewer_registry())

    def _write_oracle_lock(self, root: Path) -> Path:
        return write_gate_oracle_lock(root, PROJECT_ROOT)

    def _write_reports(self, reports: Path, lock: Path) -> None:
        contract = read_object(CONTRACT_PATH)
        contract_hash = self._sha256(CONTRACT_PATH)
        lock_hash = self._sha256(lock)
        evidence_root = reports.parent
        evidence = evidence_root / "evidence"
        evidence.mkdir(exist_ok=True)
        evaluator = self._write_evaluator_manifest(evidence, contract)
        evaluator_hash = self._sha256(evaluator)
        evaluator_binding = self._binding(evidence_root, evaluator)
        strata_by_format = object_value(contract, "strata")
        quotas_by_format = object_value(contract, "stratum_quotas")
        security_by_format = object_value(contract, "security_case_outcomes")
        paired_by_format = object_value(contract, "legacy_paired_stratum_quotas")
        for document_format in string_list(contract, "required_formats"):
            paired_quotas = (
                object_value(paired_by_format, document_format)
                if document_format in PAIRED_FORMATS
                else None
            )
            corpus_path = write_corpus(
                evidence,
                document_format,
                contract_hash,
                object_value(quotas_by_format, document_format),
                object_value(security_by_format, document_format),
                paired_quotas,
            )
            metrics_path = write_metrics(
                CONTRACT_PATH,
                corpus_path,
                evaluator_hash,
                lock_hash,
                evidence_root,
            )
            report = self._passing_report(
                document_format,
                contract_hash,
                lock_hash,
                {
                    name: 100.0
                    for name in string_list(strata_by_format, document_format)
                },
                evaluator_binding,
                self._binding(evidence_root, corpus_path),
                self._binding(evidence_root, metrics_path),
            )
            (reports / f"{document_format}.json").write_text(
                json.dumps(report, sort_keys=True),
                encoding="utf-8",
            )

    def _write_evaluator_manifest(
        self,
        evidence: Path,
        contract: dict[str, JsonValue],
    ) -> Path:
        evaluator_lock = read_object(
            PROJECT_ROOT / "evaluate" / "multiformat" / "evaluator-lock.v1.json"
        )
        path = evidence / "evaluator-manifest.json"
        path.write_text(
            json.dumps(
                {
                    "schema_version": 2,
                    "contract_sha256": self._sha256(CONTRACT_PATH),
                    "project_revision": current_project_revision(PROJECT_ROOT),
                    "python": evaluator_lock["python"],
                    "unicode_version": evaluator_lock["unicode_version"],
                    "algorithm_parameters": object_value(
                        contract,
                        "metric_parameters",
                    ),
                    "dependencies": object_value(evaluator_lock, "dependencies"),
                    "files": [
                        {
                            "path": relative_path,
                            "sha256": self._sha256(PROJECT_ROOT / relative_path),
                        }
                        for relative_path in EVALUATOR_FILES
                    ],
                },
                sort_keys=True,
            ),
            encoding="utf-8",
        )
        return path

    @staticmethod
    def _passing_report(
        document_format: str,
        contract_hash: str,
        lock_hash: str,
        strata: dict[str, float],
        evaluator: dict[str, str],
        corpus_manifest: dict[str, str],
        metrics_evidence: dict[str, str],
    ) -> dict[str, JsonValue]:
        return {
            "schema_version": 2,
            "status": "READY",
            "format": document_format,
            "contract_sha256": contract_hash,
            "oracle_lock_sha256": lock_hash,
            "evaluator": evaluator,
            "corpus_manifest": corpus_manifest,
            "metrics_evidence": metrics_evidence,
            "conformance": {
                "score": 100.0,
                "visual": 100.0,
                "content": 100.0,
                "layout": 100.0,
                "unit_count": 100,
                "minimum_unit_score": 100.0,
                "critical_defects": 0,
                "strata": strata,
            },
            "blind": {
                "score": 100.0,
                "visual": 100.0,
                "content": 100.0,
                "layout": 100.0,
                "file_count": 75,
                "accepted_files": 75,
                "critical_defects": 0,
                "minimum_file_score": 100.0,
            },
            "security": {"case_count": 10, "passed": 10},
            "determinism": {
                "runs": 2,
                "html_hashes_equal": True,
                "inventory_hashes_equal": True,
                "png_hashes_equal": True,
            },
            "review": {"reviewers": 2, "all_passed": True},
            "quality": {
                "tests_passed": True,
                "builds_passed": True,
                "diagnostics_passed": True,
                "contract_checks_passed": True,
            },
            "performance": {"within_limits": True},
        }

    @classmethod
    def _binding(cls, root: Path, path: Path) -> dict[str, str]:
        return {
            "path": path.relative_to(root).as_posix(),
            "sha256": cls._sha256(path),
        }

    @staticmethod
    def _sha256(path: Path) -> str:
        return hashlib.sha256(path.read_bytes()).hexdigest()
