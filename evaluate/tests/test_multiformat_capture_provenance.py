import hashlib
import json
import tempfile
import unittest
from pathlib import Path

from evaluate.multiformat_capture_provenance import (
    validate_portable_capture_provenance,
)
from evaluate.multiformat_metric_types import MetricError
from evaluate.multiformat_metrics import validate_metrics_evidence
from evaluate.tests.multiformat_metrics_fixture import write_metrics
from evaluate.tests.multiformat_portable_receipt_fixture import ReceiptFixture
from evaluate.tests.multiformat_small_corpus_fixture import ready_fixture


class MultiFormatCaptureProvenanceTests(unittest.TestCase):
    def test_portable_provenance_delegates_to_strict_receipt_validation(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            fixture = ReceiptFixture(Path(temp_dir))
            fixture.sign()

            verified = validate_portable_capture_provenance(
                fixture.receipt,
                fixture.verification(),
            )

            self.assertEqual(verified.nonce, "a" * 64)

    def test_outer_roles_cannot_swap_artifacts_bound_by_upstream_producers(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            contract, corpus = ready_fixture(root)
            metrics = write_metrics(contract, corpus, "e" * 64, "a" * 64)
            value = json.loads(metrics.read_text(encoding="utf-8"))
            oracle_binding = value["bindings"]["oracle_capture"]
            candidate_binding = value["bindings"]["candidate_capture"]
            oracle_path = metrics.parent / oracle_binding["path"]
            candidate_path = metrics.parent / candidate_binding["path"]
            oracle = json.loads(oracle_path.read_text(encoding="utf-8"))
            candidate = json.loads(candidate_path.read_text(encoding="utf-8"))
            oracle["units"], candidate["units"] = (
                candidate["units"],
                oracle["units"],
            )
            oracle_path.write_text(
                json.dumps(oracle, sort_keys=True),
                encoding="utf-8",
            )
            candidate_path.write_text(
                json.dumps(candidate, sort_keys=True),
                encoding="utf-8",
            )
            oracle_binding["sha256"] = self._sha256(oracle_path)
            candidate_binding["sha256"] = self._sha256(candidate_path)
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

    @staticmethod
    def _sha256(path: Path) -> str:
        return hashlib.sha256(path.read_bytes()).hexdigest()
