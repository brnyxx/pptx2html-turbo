import json
import tempfile
import unittest
from pathlib import Path

from evaluate.assemble_multiformat_report import assemble_report
from evaluate.tests.multiformat_gate_fixture import (
    CONTRACT_PATH,
    PROJECT_ROOT,
    MultiFormatGateFixture,
)


class AssembleMultiFormatReportTests(MultiFormatGateFixture, unittest.TestCase):
    def test_assembler_reproduces_the_only_gate_accepted_report(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            reports = root / "reports"
            reports.mkdir()
            lock = self._write_oracle_lock(root)
            self._write_reports(reports, lock)
            expected = json.loads((reports / "docx.json").read_text(encoding="utf-8"))

            actual = assemble_report(
                PROJECT_ROOT,
                CONTRACT_PATH,
                lock,
                root / expected["evaluator"]["path"],
                root / expected["corpus_manifest"]["path"],
                root / expected["metrics_evidence"]["path"],
                root,
            )

            self.assertEqual(actual, expected)
