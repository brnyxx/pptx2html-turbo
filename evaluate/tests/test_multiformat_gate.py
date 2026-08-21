import json
import tempfile
import unittest
from pathlib import Path

from evaluate.multiformat_gate import GateStatus, evaluate_reports
from evaluate.tests.multiformat_gate_fixture import (
    CONTRACT_PATH,
    MultiFormatGateFixture,
)


class MultiFormatGateTests(MultiFormatGateFixture, unittest.TestCase):
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
            lock = self._write_oracle_lock(root)
            self._write_reports(reports, lock)
            lock.write_text(
                json.dumps({"schema_version": 1, "status": "locked"}),
                encoding="utf-8",
            )

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
            report["metrics_evidence"]["sha256"] = "not-a-digest"
            report_path.write_text(json.dumps(report, sort_keys=True), encoding="utf-8")

            # When
            summary = evaluate_reports(CONTRACT_PATH, reports, lock)

            # Then
            self.assertEqual(summary.status, GateStatus.FAIL)
            pdf = next(result for result in summary.formats if result.format == "pdf")
            self.assertEqual(pdf.reasons, ("metrics_evidence",))

    def test_missing_bound_evidence_file_fails_closed(self) -> None:
        # Given
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            reports = root / "reports"
            reports.mkdir()
            lock = self._write_oracle_lock(root)
            self._write_reports(reports, lock)
            report = json.loads((reports / "pptx.json").read_text(encoding="utf-8"))
            (root / report["metrics_evidence"]["path"]).unlink()

            # When
            summary = evaluate_reports(CONTRACT_PATH, reports, lock)

            # Then
            self.assertEqual(summary.status, GateStatus.FAIL)
            pptx = next(result for result in summary.formats if result.format == "pptx")
            self.assertEqual(pptx.reasons, ("metrics_evidence",))

    def test_evidence_path_cannot_escape_root(self) -> None:
        # Given
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            reports = root / "reports"
            reports.mkdir()
            lock = self._write_oracle_lock(root)
            self._write_reports(reports, lock)
            report_path = reports / "pptx.json"
            report = json.loads(report_path.read_text(encoding="utf-8"))
            report["evaluator"]["path"] = "../oracle-lock.json"
            report_path.write_text(json.dumps(report, sort_keys=True), encoding="utf-8")

            # When
            summary = evaluate_reports(CONTRACT_PATH, reports, lock)

            # Then
            self.assertEqual(summary.status, GateStatus.FAIL)
            pptx = next(result for result in summary.formats if result.format == "pptx")
            self.assertEqual(pptx.reasons, ("evaluator",))

    def test_ready_report_cannot_bind_an_incomplete_corpus(self) -> None:
        # Given
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            reports = root / "reports"
            reports.mkdir()
            lock = self._write_oracle_lock(root)
            self._write_reports(reports, lock)
            corpus_path = root / "evidence" / "corpora" / "pdf" / "manifest.json"
            corpus = json.loads(corpus_path.read_text(encoding="utf-8"))
            corpus["status"] = "INCOMPLETE"
            for track in corpus["tracks"].values():
                track["items"] = []
            corpus_path.write_text(json.dumps(corpus, sort_keys=True), encoding="utf-8")
            report_path = reports / "pdf.json"
            report = json.loads(report_path.read_text(encoding="utf-8"))
            report["corpus_manifest"]["sha256"] = self._sha256(corpus_path)
            report_path.write_text(json.dumps(report, sort_keys=True), encoding="utf-8")

            # When
            summary = evaluate_reports(CONTRACT_PATH, reports, lock)

            # Then
            self.assertEqual(summary.status, GateStatus.FAIL)
            pdf = next(result for result in summary.formats if result.format == "pdf")
            self.assertIn("corpus_manifest", pdf.reasons)

    def test_legacy_corpus_requires_a_bound_modern_pair(self) -> None:
        # Given
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            reports = root / "reports"
            reports.mkdir()
            lock = self._write_oracle_lock(root)
            self._write_reports(reports, lock)
            corpus_path = root / "evidence" / "corpora" / "doc" / "manifest.json"
            corpus = json.loads(corpus_path.read_text(encoding="utf-8"))
            corpus["tracks"]["conformance"]["items"][0]["paired_source"] = None
            corpus_path.write_text(json.dumps(corpus, sort_keys=True), encoding="utf-8")
            report_path = reports / "doc.json"
            report = json.loads(report_path.read_text(encoding="utf-8"))
            report["corpus_manifest"]["sha256"] = self._sha256(corpus_path)
            report_path.write_text(json.dumps(report, sort_keys=True), encoding="utf-8")

            # When
            summary = evaluate_reports(CONTRACT_PATH, reports, lock)

            # Then
            self.assertEqual(summary.status, GateStatus.FAIL)
            doc = next(result for result in summary.formats if result.format == "doc")
            self.assertIn("corpus_manifest", doc.reasons)

    def test_legacy_paired_labels_cannot_move_to_binary_units(self) -> None:
        # Given
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            reports = root / "reports"
            reports.mkdir()
            lock = self._write_oracle_lock(root)
            self._write_reports(reports, lock)
            corpus_path = root / "evidence" / "corpora" / "doc" / "manifest.json"
            corpus = json.loads(corpus_path.read_text(encoding="utf-8"))
            items = corpus["tracks"]["conformance"]["items"]
            moved_stratum = items[0]["units"][0]["paired_stratum"]
            items[0]["units"][0]["paired_stratum"] = None
            items[1]["units"][0]["paired_stratum"] = moved_stratum
            corpus_path.write_text(json.dumps(corpus, sort_keys=True), encoding="utf-8")
            report_path = reports / "doc.json"
            report = json.loads(report_path.read_text(encoding="utf-8"))
            report["corpus_manifest"]["sha256"] = self._sha256(corpus_path)
            report_path.write_text(json.dumps(report, sort_keys=True), encoding="utf-8")

            # When
            summary = evaluate_reports(CONTRACT_PATH, reports, lock)

            # Then
            self.assertEqual(summary.status, GateStatus.FAIL)
            doc = next(result for result in summary.formats if result.format == "doc")
            self.assertIn("corpus_manifest", doc.reasons)
