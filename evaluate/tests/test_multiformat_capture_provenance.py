import hashlib
import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from evaluate.multiformat_capture_manifest import validate_capture_manifest
from evaluate.multiformat_capture_profile import load_capture_profile
from evaluate.multiformat_capture_provenance import (
    validate_portable_capture_provenance,
)
from evaluate.multiformat_metric_types import MetricError
from evaluate.multiformat_metrics import validate_metrics_evidence
from evaluate.tests.multiformat_metrics_fixture import write_metrics
from evaluate.tests.multiformat_portable_capture_fixture import (
    PortableCaptureFixture,
)
from evaluate.tests.multiformat_portable_receipt_fixture import ReceiptFixture
from evaluate.tests.multiformat_small_corpus_fixture import ready_fixture


class MultiFormatCaptureProvenanceTests(unittest.TestCase):
    def test_schema_2_oracle_without_office_batch_uses_no_office_validator(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            fixture = PortableCaptureFixture(Path(temp_dir), "oracle")

            with (
                mock.patch(
                    "evaluate.multiformat_capture_runtime.validate_office_oracle_runtime"
                ) as runtime_validator,
                mock.patch(
                    "evaluate.multiformat_capture_provenance."
                    "validate_office_oracle_provenance"
                ) as provenance_validator,
            ):
                validated = validate_capture_manifest(*fixture.validate_arguments())

            self.assertEqual(set(validated.units), {"unit-1"})
            runtime_validator.assert_not_called()
            provenance_validator.assert_not_called()

    def test_schema_2_candidate_uses_bound_runtime_lock_and_portable_receipt(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            fixture = PortableCaptureFixture(Path(temp_dir), "candidate")

            validated = validate_capture_manifest(*fixture.validate_arguments())

            self.assertEqual(set(validated.files), {"source"})

    def test_schema_2_candidate_runtime_lock_is_admitted(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            candidate_lock = root / "candidate-runtime-lock.json"
            candidate_lock.write_text(
                json.dumps({"schema_version": 2, "status": "locked"}),
                encoding="utf-8",
            )
            fixture = ReceiptFixture(
                root,
                candidate_runtime_lock=candidate_lock,
            )

            profile = load_capture_profile(
                fixture.lock,
                fixture.trust.lock_sha256,
                root,
                "candidate",
            )

            self.assertEqual(profile.candidate_lock_path, candidate_lock.resolve())

    def test_portable_receipt_binds_units_runtime_and_execution(self) -> None:
        for field in ("png", "inventory", "runtime_identity", "execution_log"):
            with self.subTest(field=field), tempfile.TemporaryDirectory() as temp_dir:
                fixture = PortableCaptureFixture(Path(temp_dir), "oracle")
                fixture.replace_bound_artifact(field)

                with self.assertRaisesRegex(MetricError, "portable receipt artifacts"):
                    validate_capture_manifest(*fixture.validate_arguments())

    def test_schema_2_candidate_runtime_lock_tamper_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            fixture = PortableCaptureFixture(Path(temp_dir), "candidate")
            candidate_lock = fixture.receipt_fixture.candidate_runtime_lock
            self.assertIsNotNone(candidate_lock)
            assert candidate_lock is not None
            candidate_lock.write_text("{}", encoding="utf-8")

            with self.assertRaises(MetricError):
                validate_capture_manifest(*fixture.validate_arguments())

    def test_portable_provenance_delegates_to_strict_receipt_validation(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            fixture = ReceiptFixture(Path(temp_dir))
            fixture.sign()

            verified = validate_portable_capture_provenance(
                fixture.receipt,
                fixture.verification(),
            )

            self.assertEqual(verified.nonce, fixture.nonce)

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
