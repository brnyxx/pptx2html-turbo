from __future__ import annotations

import hashlib
import json
from pathlib import Path

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

PROJECT_ROOT = Path(__file__).resolve().parents[2]
CONTRACT_PATH = PROJECT_ROOT / "evaluate" / "multiformat" / "contract.v1.json"


class MultiFormatGateFixture:
    def _write_oracle_lock(self, root: Path) -> Path:
        lock = root / "oracle-lock.json"
        lock.write_text(
            json.dumps(
                {
                    "schema_version": 1,
                    "status": "locked",
                    "office": {
                        "os": "Windows 11 23H2",
                        "word": "test-build",
                        "excel": "test-build",
                        "powerpoint": "test-build",
                    },
                    "pdf": {"primary": "test-mupdf", "secondary": "test-renderer"},
                    "browser": {"chromium": "test-revision"},
                    "font_bundle_sha256": "a" * 64,
                },
                sort_keys=True,
            ),
            encoding="utf-8",
        )
        return lock

    def _write_reports(self, reports: Path, lock: Path) -> None:
        contract = read_object(CONTRACT_PATH)
        contract_hash = self._sha256(CONTRACT_PATH)
        lock_hash = self._sha256(lock)
        evidence = reports.parent / "evidence"
        evidence.mkdir(exist_ok=True)
        evaluator = evidence / "evaluator.py"
        evaluator.write_text("evaluator\n", encoding="utf-8")
        metrics = evidence / "metrics.json"
        metrics.write_text("metrics\n", encoding="utf-8")
        shared_bindings = {
            "evaluator": self._binding(reports.parent, evaluator),
            "metrics_evidence": self._binding(reports.parent, metrics),
        }
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
            bindings = {
                **shared_bindings,
                "corpus_manifest": self._binding(reports.parent, corpus_path),
            }
            strata = {
                name: 96.5 for name in string_list(strata_by_format, document_format)
            }
            report = self._passing_report(
                document_format,
                contract_hash,
                lock_hash,
                strata,
                bindings,
            )
            (reports / f"{document_format}.json").write_text(
                json.dumps(report, sort_keys=True),
                encoding="utf-8",
            )

    @staticmethod
    def _passing_report(
        document_format: str,
        contract_hash: str,
        lock_hash: str,
        strata: dict[str, float],
        bindings: dict[str, dict[str, str]],
    ) -> dict[str, JsonValue]:
        track = {
            "score": 96.5,
            "visual": 96.0,
            "content": 98.5,
            "layout": 95.0,
        }
        return {
            "schema_version": 1,
            "status": "READY",
            "format": document_format,
            "contract_sha256": contract_hash,
            "oracle_lock_sha256": lock_hash,
            "evaluator": bindings["evaluator"],
            "corpus_manifest": bindings["corpus_manifest"],
            "metrics_evidence": bindings["metrics_evidence"],
            "conformance": {
                **track,
                "unit_count": 100,
                "minimum_unit_score": 90.0,
                "strata": strata,
            },
            "blind": {
                **track,
                "file_count": 75,
                "accepted_files": 75,
                "critical_defects": 0,
                "minimum_file_score": 92.0,
            },
            "security": {"case_count": 10, "passed": 10},
            "determinism": {
                "runs": 2,
                "html_hashes_equal": True,
                "inventory_hashes_equal": True,
                "png_hashes_equal": True,
            },
            "review": {"reviewers": 2, "all_passed": True},
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
