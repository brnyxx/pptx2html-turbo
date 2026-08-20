import json
import tempfile
import unittest
from pathlib import Path

from evaluate.multiformat_gate import GateStatus, evaluate_reports
from evaluate.multiformat_schema import JsonValue
from evaluate.tests.multiformat_gate_fixture import (
    CONTRACT_PATH,
    MultiFormatGateFixture,
)
from evaluate.tests.multiformat_metric_artifact_fixture import write_checkerboard_png


class MultiFormatMetricsGateTests(MultiFormatGateFixture, unittest.TestCase):
    def test_report_cannot_hide_a_low_raw_unit_score(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root, reports, lock = self._fixture(Path(temp_dir))
            report_path = reports / "docx.json"
            report = json.loads(report_path.read_text(encoding="utf-8"))
            metrics_path = root / report["metrics_evidence"]["path"]
            metrics = json.loads(metrics_path.read_text(encoding="utf-8"))
            candidate = metrics["conformance"]["units"][0]["artifacts"]["candidate_png"]
            candidate_path = root / candidate["path"]
            write_checkerboard_png(candidate_path, 192, 192)
            candidate["sha256"] = self._sha256(candidate_path)
            capture_binding = metrics["bindings"]["candidate_capture"]
            capture_path = root / capture_binding["path"]
            capture = json.loads(capture_path.read_text(encoding="utf-8"))
            capture_unit = next(
                unit
                for unit in capture["units"]
                if unit["unit_id"] == metrics["conformance"]["units"][0]["unit_id"]
            )
            capture_unit["png"]["sha256"] = candidate["sha256"]
            upstream_binding = capture["upstream_manifest"]
            upstream_path = root / upstream_binding["path"]
            upstream = json.loads(upstream_path.read_text(encoding="utf-8"))
            upstream_unit = next(
                unit
                for unit in upstream["units"]
                if unit["unit_id"] == capture_unit["unit_id"]
            )
            upstream_unit["png"]["sha256"] = candidate["sha256"]
            upstream_path.write_text(
                json.dumps(upstream, sort_keys=True),
                encoding="utf-8",
            )
            upstream_binding["sha256"] = self._sha256(upstream_path)
            capture_path.write_text(
                json.dumps(capture, sort_keys=True),
                encoding="utf-8",
            )
            capture_binding["sha256"] = self._sha256(capture_path)
            source_id = metrics["conformance"]["units"][0]["source_id"]
            run_file = next(
                item
                for item in metrics["determinism"]["runs"][0]["files"]
                if item["source_id"] == source_id
            )
            run_file["png"][0]["sha256"] = candidate["sha256"]
            unit_id = metrics["conformance"]["units"][0]["unit_id"]
            for reviewer in metrics["review"]["reviewers"]:
                attestation = reviewer["attestation"]
                attestation_path = root / attestation["path"]
                review = json.loads(attestation_path.read_text(encoding="utf-8"))
                pair = next(
                    item for item in review["pairs"] if item["pair_id"] == unit_id
                )
                pair["candidate_png_sha256"] = candidate["sha256"]
                attestation_path.write_text(
                    json.dumps(review, sort_keys=True),
                    encoding="utf-8",
                )
                attestation["sha256"] = self._sha256(attestation_path)
            self._rewrite_metrics(metrics_path, metrics, report_path, report)

            summary = evaluate_reports(CONTRACT_PATH, reports, lock)

            self.assertEqual(summary.status, GateStatus.FAIL)
            docx = next(result for result in summary.formats if result.format == "docx")
            self.assertIn("report.aggregate_mismatch", docx.reasons)
            self.assertIn("conformance.minimum_unit_score", docx.reasons)

    def test_determinism_is_recomputed_from_per_file_hashes(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root, reports, lock = self._fixture(Path(temp_dir))
            report_path = reports / "pdf.json"
            report = json.loads(report_path.read_text(encoding="utf-8"))
            metrics_path = root / report["metrics_evidence"]["path"]
            metrics = json.loads(metrics_path.read_text(encoding="utf-8"))
            html = metrics["determinism"]["runs"][1]["files"][0]["html"]
            html_path = root / html["path"]
            html_path.write_text("<html>changed</html>", encoding="utf-8")
            html["sha256"] = self._sha256(html_path)
            self._rewrite_metrics(metrics_path, metrics, report_path, report)

            summary = evaluate_reports(CONTRACT_PATH, reports, lock)

            pdf = next(result for result in summary.formats if result.format == "pdf")
            self.assertIn("determinism", pdf.reasons)
            self.assertIn("report.aggregate_mismatch", pdf.reasons)

    def test_each_reviewer_must_cover_every_pair(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root, reports, lock = self._fixture(Path(temp_dir))
            report_path = reports / "xlsx.json"
            report = json.loads(report_path.read_text(encoding="utf-8"))
            metrics_path = root / report["metrics_evidence"]["path"]
            metrics = json.loads(metrics_path.read_text(encoding="utf-8"))
            attestation = metrics["review"]["reviewers"][0]["attestation"]
            attestation_path = root / attestation["path"]
            review = json.loads(attestation_path.read_text(encoding="utf-8"))
            review["pairs"].pop()
            attestation_path.write_text(
                json.dumps(review, sort_keys=True),
                encoding="utf-8",
            )
            attestation["sha256"] = self._sha256(attestation_path)
            self._rewrite_metrics(metrics_path, metrics, report_path, report)

            summary = evaluate_reports(CONTRACT_PATH, reports, lock)

            xlsx = next(result for result in summary.formats if result.format == "xlsx")
            self.assertIn("review", xlsx.reasons)

    def _fixture(self, root: Path) -> tuple[Path, Path, Path]:
        reports = root / "reports"
        reports.mkdir()
        lock = self._write_oracle_lock(root)
        self._write_reports(reports, lock)
        return root, reports, lock

    def _rewrite_metrics(
        self,
        metrics_path: Path,
        metrics: dict[str, JsonValue],
        report_path: Path,
        report: dict[str, JsonValue],
    ) -> None:
        metrics_path.write_text(
            json.dumps(metrics, sort_keys=True),
            encoding="utf-8",
        )
        report["metrics_evidence"]["sha256"] = self._sha256(metrics_path)
        report_path.write_text(
            json.dumps(report, sort_keys=True),
            encoding="utf-8",
        )
