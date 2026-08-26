import json
import tempfile
import unittest
from dataclasses import replace
from pathlib import Path
from unittest import mock

from evaluate.multiformat_candidate_preflight import (
    CandidatePreflightError,
    preflight_candidate_capture,
)
from evaluate.multiformat_candidate_runtime_profile import (
    legacy_candidate_runtime_profile,
)
from evaluate.multiformat_candidate_types import CandidateCaptureError
from evaluate.tests.multiformat_candidate_pipeline_fixture import (
    PipelineFixture,
    prepare_pipeline_fixture,
)

PROJECT_ROOT = Path(__file__).resolve().parents[2]


class MultiFormatCandidatePreflightTests(unittest.TestCase):
    def test_rejects_unproven_golden_isolation(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            fixture = prepare_pipeline_fixture(Path(temp_dir), PROJECT_ROOT)
            attestation = json.loads(
                fixture.sandbox_attestation.read_text(encoding="utf-8")
            )
            attestation["golden_access"] = "allowed"
            fixture.sandbox_attestation.write_text(
                json.dumps(attestation, sort_keys=True),
                encoding="utf-8",
            )

            with self.assertRaisesRegex(
                CandidatePreflightError,
                "sandbox attestation",
            ):
                self._preflight(fixture)

    def test_rejects_nonempty_candidate_output(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            fixture = prepare_pipeline_fixture(Path(temp_dir), PROJECT_ROOT)
            fixture.output.mkdir()
            (fixture.output / "stale").write_text("stale", encoding="utf-8")

            with self.assertRaisesRegex(
                CandidatePreflightError,
                "not empty",
            ):
                self._preflight(fixture)

    def test_native_package_profile_always_runs_package_validation(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            # Given: a native-package profile whose runtime mapping omits the trigger field.
            fixture = prepare_pipeline_fixture(Path(temp_dir), PROJECT_ROOT)
            profile = legacy_candidate_runtime_profile(fixture.oracle_lock)
            native_profile = replace(profile, native_packages=True)

            # When/Then: profile-driven validation runs despite the missing field.
            with (
                mock.patch(
                    "evaluate.multiformat_candidate_preflight.oracle_lock_ready",
                    return_value=True,
                ),
                mock.patch(
                    "evaluate.multiformat_candidate_preflight.validate_evaluator_manifest"
                ),
                mock.patch(
                    "evaluate.multiformat_candidate_preflight.resolve_candidate_runtime_profile",
                    return_value=native_profile,
                ),
                mock.patch(
                    "evaluate.multiformat_candidate_preflight.validate_candidate_native_packages",
                    side_effect=CandidateCaptureError("native-package-validation-ran"),
                ),
                self.assertRaisesRegex(
                    CandidatePreflightError, "native-package-validation-ran"
                ),
            ):
                self._preflight(fixture)

    def test_rejects_converter_not_bound_by_runtime_lock(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            fixture = prepare_pipeline_fixture(Path(temp_dir), PROJECT_ROOT)
            lock = json.loads(fixture.oracle_lock.read_text(encoding="utf-8"))
            lock["candidate_runtime"]["converter_sha256"] = "0" * 64
            fixture.oracle_lock.write_text(
                json.dumps(lock, sort_keys=True),
                encoding="utf-8",
            )

            with self.assertRaisesRegex(
                CandidatePreflightError,
                "converter executable hash",
            ):
                self._preflight(fixture)

    @staticmethod
    def _preflight(fixture: PipelineFixture) -> None:
        preflight_candidate_capture(
            PROJECT_ROOT,
            fixture.contract,
            fixture.corpus,
            fixture.evaluator,
            fixture.oracle_lock,
            fixture.evidence_root,
            fixture.output,
            converter=fixture.converter,
            soffice=fixture.soffice,
            pdftohtml=fixture.pdftohtml,
            pdfinfo=fixture.pdfinfo,
            chromium=fixture.chromium,
            font_bundle=fixture.font_bundle,
            sandbox_attestation=fixture.sandbox_attestation,
            sandbox_public_key=fixture.sandbox_public_key,
            openssl=fixture.openssl,
            receipt_signer=fixture.receipt_signer,
            timeout_seconds=30,
            require_clean_worktree=False,
            require_release_binary=False,
        )
