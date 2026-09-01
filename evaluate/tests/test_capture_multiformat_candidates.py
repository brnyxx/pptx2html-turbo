import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from evaluate.multiformat_candidate_capture import capture_candidate_evidence
from evaluate.multiformat_candidate_sources import CandidateSourceSet
from evaluate.multiformat_candidate_types import (
    CandidateCaptureError,
    CandidateRun,
    CandidateRuntimePaths,
)
from evaluate.tests.multiformat_candidate_pipeline_fixture import (
    prepare_pipeline_fixture,
)

PROJECT_ROOT = Path(__file__).resolve().parents[2]


class CaptureMultiFormatCandidatesTests(unittest.TestCase):
    def test_two_clean_runs_publish_self_validating_candidate_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            fixture = prepare_pipeline_fixture(Path(temp_dir), PROJECT_ROOT)
            converter_alias = Path(temp_dir) / "converter-alias"
            converter_alias.hardlink_to(fixture.converter)

            result = capture_candidate_evidence(
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

            capture = json.loads(result.capture.read_text(encoding="utf-8"))
            determinism = json.loads(result.determinism.read_text(encoding="utf-8"))
            runtime = json.loads(result.runtime_identity.read_text(encoding="utf-8"))
            converter_snapshot = (
                fixture.evidence_root / runtime["artifacts"]["converter_binary"]["path"]
            )
            self.assertGreater(fixture.converter.stat().st_nlink, 1)
            self.assertEqual(converter_snapshot.stat().st_nlink, 1)
            self.assertNotEqual(
                (converter_snapshot.stat().st_dev, converter_snapshot.stat().st_ino),
                (fixture.converter.stat().st_dev, fixture.converter.stat().st_ino),
            )
            self.assertEqual(capture["status"], "READY")
            self.assertEqual(len(capture["units"]), 7)
            self.assertEqual(len(capture["files"]), 6)
            self.assertEqual(len(determinism["runs"]), 2)
            self.assertNotEqual(
                determinism["runs"][0]["files"][0]["inventory"]["path"],
                determinism["runs"][1]["files"][0]["inventory"]["path"],
            )

    def test_restored_runtime_mutation_during_run_fails_capture(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            fixture = prepare_pipeline_fixture(Path(temp_dir), PROJECT_ROOT)
            from evaluate.multiformat_candidate_capture import capture_clean_run

            def mutate_and_restore(
                run_id: int,
                source_set: CandidateSourceSet,
                run_root: Path,
                evidence_root: Path,
                runtime: CandidateRuntimePaths,
            ) -> CandidateRun:
                run = capture_clean_run(
                    run_id,
                    source_set,
                    run_root,
                    evidence_root,
                    runtime,
                )
                converter = runtime.converter
                original = converter.read_bytes()
                original_mode = converter.stat().st_mode
                converter.chmod(original_mode | 0o200)
                converter.write_bytes(b"attacker-runtime")
                converter.write_bytes(original)
                converter.chmod(original_mode)
                return run

            with (
                patch(
                    "evaluate.multiformat_candidate_capture.capture_clean_run",
                    side_effect=mutate_and_restore,
                ),
                patch(
                    "evaluate.multiformat_candidate_preflight.oracle_lock_ready",
                    return_value=True,
                ),
                patch(
                    "evaluate.multiformat_candidate_preflight.validate_evaluator_manifest"
                ),
                self.assertRaisesRegex(
                    CandidateCaptureError, "runtime snapshot changed"
                ),
            ):
                capture_candidate_evidence(
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
