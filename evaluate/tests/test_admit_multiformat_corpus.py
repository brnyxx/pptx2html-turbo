from __future__ import annotations

import io
import json
import stat
import subprocess
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path

from evaluate.admit_multiformat_corpus import main
from evaluate.tests.test_multiformat_corpus_admission import write_input_corpora

PROJECT_ROOT = Path(__file__).resolve().parents[2]
CONTRACT = PROJECT_ROOT / "evaluate" / "multiformat" / "contract.v1.json"


class AdmitMultiFormatCorpusCliTests(unittest.TestCase):
    def test_admit_with_qualification_commands_publishes_ready(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            manifests = write_input_corpora(root)
            qualifier = root / "qualify"
            qualifier.write_text(
                "#!/bin/sh\nprintf '{\"qualified\":true}\\n'\n",
                encoding="utf-8",
            )
            qualifier.chmod(qualifier.stat().st_mode | stat.S_IXUSR)
            destination = root / "admitted"
            command = [
                sys.executable,
                "-m",
                "evaluate.admit_multiformat_corpus",
                "admit",
                "--contract",
                str(CONTRACT),
                "--destination",
                str(destination),
                "--corpus-revision",
                "cli-qualified-v1",
                "--project-revision",
                "c" * 40,
                "--admitted-at",
                "2026-08-21T00:00:00Z",
                "--extraction-command",
                str(qualifier),
                "--font-command",
                str(qualifier),
                "--render-command",
                str(qualifier),
            ]
            for manifest in manifests:
                command.extend(("--manifest", str(manifest)))

            result = subprocess.run(
                command,
                check=False,
                capture_output=True,
                text=True,
                timeout=180,
            )

            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            payload = json.loads(result.stdout)
            self.assertEqual(payload["status"], "READY")
            self.assertTrue((destination / "READY").is_file())
            self.assertTrue((destination / "manifest.json").is_file())

    def test_validate_reports_incomplete_before_ready(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            output = io.StringIO()
            with redirect_stdout(output):
                exit_code = main(
                    [
                        "validate",
                        "--contract",
                        str(CONTRACT),
                        "--corpus-root",
                        temp_dir,
                    ]
                )

            payload = json.loads(output.getvalue())
            self.assertEqual(exit_code, 2)
            self.assertEqual(payload["status"], "INCOMPLETE")

    def test_validate_reports_fail_after_malformed_ready(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            (root / "READY").write_text("READY\n", encoding="ascii")
            (root / "manifest.json").write_text("{}", encoding="utf-8")
            output = io.StringIO()
            with redirect_stdout(output):
                exit_code = main(
                    [
                        "validate",
                        "--contract",
                        str(CONTRACT),
                        "--corpus-root",
                        temp_dir,
                    ]
                )

            payload = json.loads(output.getvalue())
            self.assertEqual(exit_code, 1)
            self.assertEqual(payload["status"], "FAIL")


if __name__ == "__main__":
    unittest.main()
