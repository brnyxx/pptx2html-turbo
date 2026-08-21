from __future__ import annotations

import unittest
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = PROJECT_ROOT / ".github" / "workflows" / "capture-office-oracles.yml"
PIPELINE = PROJECT_ROOT / "evaluate" / "run_multiformat_office_oracle_pipeline.ps1"


class MultiFormatOfficeOracleWorkflowTests(unittest.TestCase):
    def test_workflow_requires_dedicated_self_hosted_office_runner(self) -> None:
        workflow = WORKFLOW.read_text(encoding="utf-8")

        self.assertIn("workflow_dispatch:", workflow)
        self.assertIn("self-hosted", workflow)
        self.assertIn("office-oracle", workflow)
        self.assertIn("OFFICE_ORACLE_CAPTURE_WRAPPER", workflow)
        self.assertNotIn("windows-latest", workflow)
        self.assertIn("actions/checkout@v6", workflow)
        self.assertIn("actions/download-artifact@v8", workflow)
        self.assertIn("actions/upload-artifact@v7", workflow)
        self.assertIn("persist-credentials: false", workflow)
        self.assertIn("contents: read", workflow)
        self.assertNotIn("pip install", workflow)

    def test_pipeline_captures_once_then_finalizes_every_required_format(self) -> None:
        pipeline = PIPELINE.read_text(encoding="utf-8")

        self.assertIn("capture_multiformat_office_oracles.ps1", pipeline)
        self.assertIn("finalize_multiformat_office_oracles", pipeline)
        self.assertIn("required_formats", pipeline)
        self.assertIn("Get-ScopedNonce", pipeline)
        self.assertIn("OFFICE_ORACLE_RECEIPT_SIGNER", pipeline)
        self.assertIn("OFFICE_ORACLE_PUBLIC_KEY", pipeline)
        self.assertIn("OFFICE_ORACLE_OPENSSL", pipeline)
