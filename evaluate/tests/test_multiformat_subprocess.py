from __future__ import annotations

import os
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from evaluate import multiformat_subprocess
from evaluate.multiformat_candidate_process import (
    CandidateProcessError,
    CandidateProcessFailure,
    run_bounded_process,
)


class MultiFormatSubprocessTests(unittest.TestCase):
    def test_windows_environment_preserves_system_root(self) -> None:
        with (
            patch.object(os, "name", "nt"),
            patch.dict(
                os.environ,
                {
                    "SystemRoot": r"C:\Windows",
                    "WINDIR": r"C:\Windows",
                    "TEMP": r"C:\Temp",
                    "TMP": r"C:\Temp",
                },
                clear=True,
            ),
        ):
            environment = multiformat_subprocess.clean_subprocess_environment()

        self.assertEqual(environment["SystemRoot"], r"C:\Windows")
        self.assertEqual(environment["WINDIR"], r"C:\Windows")
        self.assertIn(r"C:\Windows\System32", environment["PATH"])

    def test_windows_environment_requires_system_root(self) -> None:
        with (
            patch.object(os, "name", "nt"),
            patch.dict(os.environ, {}, clear=True),
            self.assertRaises(OSError),
        ):
            _ = multiformat_subprocess.clean_subprocess_environment()

    def test_bounded_process_exposes_typed_timeout_reason(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            stdout, stderr = root / "stdout", root / "stderr"
            with self.assertRaises(CandidateProcessError) as raised:
                _ = run_bounded_process(
                    (sys.executable, "-c", "import time; time.sleep(2)"),
                    root,
                    {},
                    stdout,
                    stderr,
                    timeout_seconds=0.05,
                    max_log_bytes=1024,
                )
            self.assertIs(raised.exception.failure, CandidateProcessFailure.TIMEOUT)
