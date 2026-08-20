import json
import tempfile
import unittest
from pathlib import Path

from evaluate.multiformat_candidate_capture import capture_candidate_evidence
from evaluate.tests.multiformat_candidate_pipeline_fixture import (
    prepare_pipeline_fixture,
)

PROJECT_ROOT = Path(__file__).resolve().parents[2]


class CaptureMultiFormatCandidatesTests(unittest.TestCase):
    def test_two_clean_runs_publish_self_validating_candidate_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            fixture = prepare_pipeline_fixture(Path(temp_dir), PROJECT_ROOT)

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
            self.assertEqual(capture["status"], "READY")
            self.assertEqual(len(capture["units"]), 7)
            self.assertEqual(len(capture["files"]), 6)
            self.assertEqual(len(determinism["runs"]), 2)
            self.assertNotEqual(
                determinism["runs"][0]["files"][0]["inventory"]["path"],
                determinism["runs"][1]["files"][0]["inventory"]["path"],
            )
