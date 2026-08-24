from __future__ import annotations

import io
import os
import subprocess
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

    def test_bounded_process_kills_real_descendant_after_pid_ready_event(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            child_pid = root / "child.pid"
            child_script = root / "child.py"
            _ = child_script.write_text(
                "import os, signal, sys\nfrom pathlib import Path\n"
                + f"Path({str(child_pid)!r}).write_text(str(os.getpid()))\n"
                + "os.write(int(sys.argv[1]), str(os.getpid()).encode())\n"
                + "os.close(int(sys.argv[1]))\nsignal.pause()\n",
                encoding="utf-8",
            )
            parent_script = root / "parent.py"
            _ = parent_script.write_text(
                "import os, signal, subprocess, sys\n"
                + "read_fd, write_fd = os.pipe()\n"
                + f"subprocess.Popen([sys.executable, {str(child_script)!r}, str(write_fd)], pass_fds=(write_fd,))\n"
                + "os.close(write_fd)\nos.read(read_fd, 64)\nsignal.pause()\n",
                encoding="utf-8",
            )
            stdout, stderr = root / "stdout", root / "stderr"
            with self.assertRaises(CandidateProcessError) as raised:
                _ = run_bounded_process(
                    (sys.executable, parent_script.as_posix()),
                    root,
                    {"PATH": os.defpath},
                    stdout,
                    stderr,
                    timeout_seconds=0.1,
                    max_log_bytes=1024,
                )
            self.assertIs(raised.exception.failure, CandidateProcessFailure.TIMEOUT)
            descendant = int(child_pid.read_text(encoding="utf-8"))
            with self.assertRaises(ProcessLookupError):
                os.kill(descendant, 0)

    def test_reader_io_failure_is_typed_and_not_log_overflow(self) -> None:
        class BrokenReader:
            def read(self, _size: int | None = -1) -> bytes:
                raise OSError("reader failed")

            def close(self) -> None:
                return

        class BrokenProcess:
            pid: int = 123
            stdout: BrokenReader = BrokenReader()
            stderr: io.BytesIO = io.BytesIO()

            def wait(self, timeout: float | None = None) -> int:
                _ = timeout
                return 0

            def poll(self) -> int:
                return 0

        with (
            patch.object(subprocess, "Popen", return_value=BrokenProcess()),
            patch(
                "evaluate.multiformat_candidate_process._kill_process_group",
                return_value=True,
            ),
            tempfile.TemporaryDirectory() as temp_dir,
        ):
            root = Path(temp_dir)
            with self.assertRaises(CandidateProcessError) as raised:
                _ = run_bounded_process(
                    ("fake",),
                    root,
                    {},
                    root / "stdout",
                    root / "stderr",
                    timeout_seconds=1,
                    max_log_bytes=1024,
                )
        self.assertIs(raised.exception.failure, CandidateProcessFailure.READER_FAILED)

    def test_kill_failure_never_falls_back_to_unbounded_wait(self) -> None:
        class FakeProcess:
            pid: int = 123
            stdout: io.BytesIO = io.BytesIO()
            stderr: io.BytesIO = io.BytesIO()

            def wait(self, timeout: float | None = None) -> int:
                if timeout is None:
                    raise AssertionError("unbounded wait")
                raise subprocess.TimeoutExpired(("fake",), timeout)

            def poll(self) -> None:
                return None

        with (
            patch.object(
                subprocess,
                "Popen",
                return_value=FakeProcess(),
            ),
            patch(
                "evaluate.multiformat_candidate_process._kill_process_group",
                return_value=False,
            ),
            tempfile.TemporaryDirectory() as temp_dir,
        ):
            root = Path(temp_dir)
            with self.assertRaises(CandidateProcessError) as raised:
                _ = run_bounded_process(
                    ("fake",),
                    root,
                    {},
                    root / "stdout",
                    root / "stderr",
                    timeout_seconds=0.01,
                    max_log_bytes=1024,
                )
        self.assertIs(
            raised.exception.failure, CandidateProcessFailure.TERMINATION_FAILED
        )

    def test_bounded_process_exposes_typed_timeout_reason(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            stdout, stderr = root / "stdout", root / "stderr"
            with self.assertRaises(CandidateProcessError) as raised:
                _ = run_bounded_process(
                    (sys.executable, "-c", "import signal; signal.pause()"),
                    root,
                    {},
                    stdout,
                    stderr,
                    timeout_seconds=0.05,
                    max_log_bytes=1024,
                )
            self.assertIs(raised.exception.failure, CandidateProcessFailure.TIMEOUT)
