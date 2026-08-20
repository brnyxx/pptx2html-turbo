import tempfile
import unittest
from dataclasses import replace
from decimal import Decimal
from pathlib import Path

from evaluate.multiformat_metrics import validate_metrics_evidence
from evaluate.multiformat_report import acceptance_failures, build_report
from evaluate.multiformat_schema import JsonValue, read_object
from evaluate.tests.multiformat_metrics_fixture import write_metrics
from evaluate.tests.multiformat_small_corpus_fixture import ready_fixture


class MultiFormatReportTests(unittest.TestCase):
    def test_report_is_assembled_only_from_recomputed_metric_summary(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            contract, corpus = ready_fixture(root)
            metrics = write_metrics(contract, corpus, "e" * 64, "a" * 64)
            summary = validate_metrics_evidence(
                contract,
                corpus,
                metrics,
                "e" * 64,
                "a" * 64,
            )

            report = build_report(
                summary,
                "1" * 64,
                "2" * 64,
                self._binding("evaluator.json", "3"),
                self._binding("corpus.json", "4"),
                self._binding("metrics.json", "5"),
            )

            self.assertEqual(report["schema_version"], 2)
            self.assertEqual(report["conformance"]["score"], 100.0)
            self.assertEqual(report["blind"]["score"], 100.0)
            self.assertEqual(report["blind"]["accepted_files"], 5)
            self.assertEqual(report["security"]["passed"], 2)
            self.assertTrue(report["quality"]["tests_passed"])
            self.assertTrue(report["performance"]["within_limits"])

    def test_thresholds_use_retained_six_decimal_value_not_display_rounding(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            contract, corpus = ready_fixture(root)
            metrics = write_metrics(contract, corpus, "e" * 64, "a" * 64)
            summary = validate_metrics_evidence(
                contract,
                corpus,
                metrics,
                "e" * 64,
                "a" * 64,
            )
            below = replace(
                summary.conformance,
                score=Decimal("95.999999"),
            )
            summary = replace(summary, conformance=below)

            failures = acceptance_failures(summary, read_object(contract))

            self.assertIn("conformance.score", failures)

    @staticmethod
    def _binding(path: str, character: str) -> dict[str, JsonValue]:
        return {"path": path, "sha256": character * 64}
