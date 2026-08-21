from __future__ import annotations

import unittest
from unittest.mock import patch

from evaluate import multiformat_subprocess


class MultiFormatSubprocessTests(unittest.TestCase):
    def test_windows_environment_preserves_system_root(self) -> None:
        with (
            patch.object(multiformat_subprocess.os, "name", "nt"),
            patch.dict(
                multiformat_subprocess.os.environ,
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
            patch.object(multiformat_subprocess.os, "name", "nt"),
            patch.dict(multiformat_subprocess.os.environ, {}, clear=True),
            self.assertRaises(OSError),
        ):
            multiformat_subprocess.clean_subprocess_environment()
