import hashlib
import json
import tempfile
import unittest
from pathlib import Path
from typing import Any

from evaluate.multiformat_gate import GateStatus, evaluate_reports

PROJECT_ROOT = Path(__file__).resolve().parents[2]
CONTRACT_PATH = PROJECT_ROOT / "evaluate" / "multiformat" / "contract.v1.json"


class MultiFormatGateTests(unittest.TestCase):
    def test_all_formats_pass_only_with_complete_bound_evidence(self) -> None:
        # Given
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            reports = root / "reports"
            reports.mkdir()
            lock = self._write_oracle_lock(root)
            self._write_reports(reports, lock)

            # When
            summary = evaluate_reports(CONTRACT_PATH, reports, lock)

            # Then
            self.assertEqual(summary.status, GateStatus.PASS)
            self.assertEqual(len(summary.formats), 7)
            self.assertTrue(
                all(result.status is GateStatus.PASS for result in summary.formats)
            )

    def test_missing_format_is_incomplete_not_passed(self) -> None:
        # Given
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            reports = root / "reports"
            reports.mkdir()
            lock = self._write_oracle_lock(root)
            self._write_reports(reports, lock)
            (reports / "pdf.json").unlink()

            # When
            summary = evaluate_reports(CONTRACT_PATH, reports, lock)

            # Then
            self.assertEqual(summary.status, GateStatus.INCOMPLETE)
            pdf = next(result for result in summary.formats if result.format == "pdf")
            self.assertEqual(pdf.status, GateStatus.INCOMPLETE)

    def test_below_threshold_format_fails_closed(self) -> None:
        # Given
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            reports = root / "reports"
            reports.mkdir()
            lock = self._write_oracle_lock(root)
            self._write_reports(reports, lock)
            report_path = reports / "docx.json"
            report = json.loads(report_path.read_text(encoding="utf-8"))
            report["conformance"]["score"] = 95.99
            report_path.write_text(
                json.dumps(report, sort_keys=True),
                encoding="utf-8",
            )

            # When
            summary = evaluate_reports(CONTRACT_PATH, reports, lock)

            # Then
            self.assertEqual(summary.status, GateStatus.FAIL)
            docx = next(result for result in summary.formats if result.format == "docx")
            self.assertEqual(docx.status, GateStatus.FAIL)
            self.assertIn("conformance.score", docx.reasons)

    def test_placeholder_oracle_lock_is_incomplete(self) -> None:
        # Given
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            reports = root / "reports"
            reports.mkdir()
            lock = root / "oracle-lock.json"
            lock.write_text(
                json.dumps({"schema_version": 1, "status": "locked"}),
                encoding="utf-8",
            )
            self._write_reports(reports, lock)

            # When
            summary = evaluate_reports(CONTRACT_PATH, reports, lock)

            # Then
            self.assertEqual(summary.status, GateStatus.INCOMPLETE)
            self.assertTrue(
                all(result.reasons == ("oracle_lock",) for result in summary.formats),
            )

    def test_unbound_evidence_digest_fails_closed(self) -> None:
        # Given
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            reports = root / "reports"
            reports.mkdir()
            lock = self._write_oracle_lock(root)
            self._write_reports(reports, lock)
            report_path = reports / "pdf.json"
            report = json.loads(report_path.read_text(encoding="utf-8"))
            report["metrics_evidence_sha256"] = "not-a-digest"
            report_path.write_text(json.dumps(report, sort_keys=True), encoding="utf-8")

            # When
            summary = evaluate_reports(CONTRACT_PATH, reports, lock)

            # Then
            self.assertEqual(summary.status, GateStatus.FAIL)
            pdf = next(result for result in summary.formats if result.format == "pdf")
            self.assertEqual(pdf.reasons, ("report_schema",))

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
        contract = json.loads(CONTRACT_PATH.read_text(encoding="utf-8"))
        contract_hash = hashlib.sha256(CONTRACT_PATH.read_bytes()).hexdigest()
        lock_hash = hashlib.sha256(lock.read_bytes()).hexdigest()
        for document_format in contract["required_formats"]:
            strata = {name: 96.5 for name in contract["strata"][document_format]}
            report = self._passing_report(
                document_format,
                contract_hash,
                lock_hash,
                strata,
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
    ) -> dict[str, Any]:
        track = {
            "score": 96.5,
            "visual": 96.0,
            "content": 98.5,
            "layout": 95.0,
        }
        return {
            "schema_version": 1,
            "format": document_format,
            "contract_sha256": contract_hash,
            "oracle_lock_sha256": lock_hash,
            "evaluator_sha256": "b" * 64,
            "corpus_manifest_sha256": "c" * 64,
            "metrics_evidence_sha256": "d" * 64,
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
