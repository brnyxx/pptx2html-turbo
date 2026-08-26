import hashlib
import json
import tempfile
import unittest
from pathlib import Path

from evaluate.multiformat_metric_types import MetricError, MetricStatus
from evaluate.multiformat_metrics import validate_metrics_evidence
from evaluate.tests.multiformat_metrics_fixture import write_metrics
from evaluate.tests.multiformat_small_corpus_fixture import ready_fixture


class MultiFormatMetricsTests(unittest.TestCase):
    def test_ready_metrics_cross_link_every_corpus_record(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            contract, corpus = ready_fixture(root)
            metrics = write_metrics(contract, corpus, "e" * 64, "a" * 64)

            result = validate_metrics_evidence(
                contract,
                corpus,
                metrics,
                "e" * 64,
                "a" * 64,
            )

            self.assertEqual(result.status, MetricStatus.READY)
            self.assertEqual(result.conformance.count, 2)
            self.assertEqual(result.conformance.score, 100)
            self.assertEqual(result.blind.count, 5)
            self.assertEqual(result.blind.score, 100)
            self.assertEqual(result.security_passed, 2)
            self.assertTrue(result.determinism.html_hashes_equal)
            self.assertEqual(result.reviewer_count, 2)

    def test_unattributable_number_format_cannot_reach_ready(self) -> None:
        """An unsupported numFmt must block READY, not lower the score.

        The extractor records a structured refusal instead of a display value.
        If that evidence were merely carried along, the content metric would
        score an incomplete cell set and could still publish READY.
        """
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            contract, corpus = ready_fixture(root)
            metrics = write_metrics(contract, corpus, "e" * 64, "a" * 64)
            inventories = sorted(corpus.parent.glob("artifacts/*-reference.json"))
            self.assertTrue(inventories, "fixture must write reference inventories")
            for target in inventories:
                value = json.loads(target.read_text(encoding="utf-8"))
                value["unattributed_cells"] = [
                    {
                        "worksheet": "Sheet1",
                        "address": "A1",
                        "number_format": "unsupported",
                        "reason": "number format display text is not reproduced",
                    }
                ]
                target.write_text(json.dumps(value, sort_keys=True), encoding="utf-8")

            # Whether the artifact digest or the attribution gate trips first,
            # this evidence must never validate as READY.
            with self.assertRaises(MetricError):
                _ = validate_metrics_evidence(
                    contract,
                    corpus,
                    metrics,
                    "e" * 64,
                    "a" * 64,
                )

    def test_missing_or_orphaned_conformance_unit_is_rejected(self) -> None:
        for mutation in ["missing", "orphan"]:
            with self.subTest(mutation=mutation):
                with tempfile.TemporaryDirectory() as temp_dir:
                    root = Path(temp_dir)
                    contract, corpus = ready_fixture(root)
                    metrics = write_metrics(contract, corpus, "e" * 64, "a" * 64)
                    value = json.loads(metrics.read_text(encoding="utf-8"))
                    if mutation == "missing":
                        value["conformance"]["units"].pop()
                    else:
                        value["conformance"]["units"][0]["unit_id"] = "orphan"
                    metrics.write_text(
                        json.dumps(value, sort_keys=True),
                        encoding="utf-8",
                    )

                    with self.assertRaisesRegex(
                        MetricError,
                        "conformance|metrics.binding.capture",
                    ):
                        validate_metrics_evidence(
                            contract,
                            corpus,
                            metrics,
                            "e" * 64,
                            "a" * 64,
                        )

    def test_blind_unit_count_and_source_digest_are_corpus_bound(self) -> None:
        for mutation in ["unit_count", "source_sha256"]:
            with self.subTest(mutation=mutation):
                with tempfile.TemporaryDirectory() as temp_dir:
                    root = Path(temp_dir)
                    contract, corpus = ready_fixture(root)
                    metrics = write_metrics(contract, corpus, "e" * 64, "a" * 64)
                    value = json.loads(metrics.read_text(encoding="utf-8"))
                    first = value["blind"]["files"][0]
                    if mutation == "unit_count":
                        first["units"].append(dict(first["units"][0]))
                        first["units"][1]["unit_id"] = "blind-0-unit-2"
                        first["units"][1]["ordinal"] = 2
                    else:
                        first["source_sha256"] = "0" * 64
                    metrics.write_text(
                        json.dumps(value, sort_keys=True),
                        encoding="utf-8",
                    )

                    with self.assertRaisesRegex(MetricError, "blind"):
                        validate_metrics_evidence(
                            contract,
                            corpus,
                            metrics,
                            "e" * 64,
                            "a" * 64,
                        )

    def test_metrics_bind_contract_corpus_evaluator_and_oracle(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            contract, corpus = ready_fixture(root)
            metrics = write_metrics(contract, corpus, "e" * 64, "a" * 64)

            with self.assertRaisesRegex(MetricError, "metrics.evaluator"):
                validate_metrics_evidence(
                    contract,
                    corpus,
                    metrics,
                    "f" * 64,
                    "a" * 64,
                )

    def test_duplicate_json_keys_are_rejected_before_scoring(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            contract, corpus = ready_fixture(root)
            metrics = write_metrics(contract, corpus, "e" * 64, "a" * 64)
            text = metrics.read_text(encoding="utf-8")
            metrics.write_text(
                text.replace(
                    '"status": "READY"',
                    '"status": "READY", "status": "READY"',
                    1,
                ),
                encoding="utf-8",
            )

            with self.assertRaisesRegex(MetricError, "metrics.schema"):
                validate_metrics_evidence(
                    contract,
                    corpus,
                    metrics,
                    "e" * 64,
                    "a" * 64,
                )

    def test_quality_status_is_read_from_bound_execution_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            contract, corpus = ready_fixture(root)
            metrics = write_metrics(contract, corpus, "e" * 64, "a" * 64)
            value = json.loads(metrics.read_text(encoding="utf-8"))
            tests_binding = value["quality"]["tests"]
            tests_path = metrics.parent / tests_binding["path"]
            result = json.loads(tests_path.read_text(encoding="utf-8"))
            result["status"] = "FAIL"
            result["exit_code"] = 1
            tests_path.write_text(
                json.dumps(result, sort_keys=True),
                encoding="utf-8",
            )
            tests_binding["sha256"] = self._sha256(tests_path)
            metrics.write_text(json.dumps(value, sort_keys=True), encoding="utf-8")

            summary = validate_metrics_evidence(
                contract,
                corpus,
                metrics,
                "e" * 64,
                "a" * 64,
            )

            self.assertFalse(summary.quality.tests_passed)

    def test_oracle_capture_cannot_be_relabelled_as_candidate(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            contract, corpus = ready_fixture(root)
            metrics = write_metrics(contract, corpus, "e" * 64, "a" * 64)
            value = json.loads(metrics.read_text(encoding="utf-8"))
            oracle_binding = value["bindings"]["oracle_capture"]
            oracle_path = metrics.parent / oracle_binding["path"]
            oracle = json.loads(oracle_path.read_text(encoding="utf-8"))
            oracle["role"] = "candidate"
            oracle["producer"] = "document2html-candidate"
            oracle_path.write_text(
                json.dumps(oracle, sort_keys=True),
                encoding="utf-8",
            )
            oracle_binding["sha256"] = self._sha256(oracle_path)
            metrics.write_text(json.dumps(value, sort_keys=True), encoding="utf-8")

            with self.assertRaisesRegex(
                MetricError,
                "metrics.binding.capture",
            ):
                validate_metrics_evidence(
                    contract,
                    corpus,
                    metrics,
                    "e" * 64,
                    "a" * 64,
                )

    def test_security_outcome_is_parsed_from_bound_execution_result(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            contract, corpus = ready_fixture(root)
            metrics = write_metrics(contract, corpus, "e" * 64, "a" * 64)
            value = json.loads(metrics.read_text(encoding="utf-8"))
            execution = value["security"]["cases"][0]["execution"]
            execution_path = metrics.parent / execution["path"]
            result = json.loads(execution_path.read_text(encoding="utf-8"))
            result["observed_outcome"] = "safe-convert"
            result["typed_error"] = None
            execution_path.write_text(
                json.dumps(result, sort_keys=True),
                encoding="utf-8",
            )
            execution["sha256"] = self._sha256(execution_path)
            metrics.write_text(json.dumps(value, sort_keys=True), encoding="utf-8")

            summary = validate_metrics_evidence(
                contract,
                corpus,
                metrics,
                "e" * 64,
                "a" * 64,
            )

            self.assertEqual(summary.security_passed, 1)

    @staticmethod
    def _sha256(path: Path) -> str:
        return hashlib.sha256(path.read_bytes()).hexdigest()
