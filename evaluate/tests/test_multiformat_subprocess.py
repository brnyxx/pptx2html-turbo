from __future__ import annotations

import io
import os
import socket
import subprocess
import sys
import tempfile
import threading
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

    def test_normal_exit_kills_ready_descendant_without_closing_active_pipe(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            child_pid = root / "child.pid"
            parent_pid = root / "parent.pid"
            ready_socket = root / "ready.sock"
            child_script = root / "child.py"
            _ = child_script.write_text(
                "import os, signal, sys\nfrom pathlib import Path\n"
                + f"Path({str(child_pid)!r}).write_text(str(os.getpid()))\n"
                + "os.write(int(sys.argv[2]), b'ready')\n"
                + "os.close(int(sys.argv[2]))\nsignal.pause()\n",
                encoding="utf-8",
            )
            parent_script = root / "parent.py"
            _ = parent_script.write_text(
                "import os, socket, subprocess, sys\nfrom pathlib import Path\n"
                + f"Path({str(parent_pid)!r}).write_text(str(os.getpid()))\n"
                + "read_fd, write_fd = os.pipe()\n"
                + f"subprocess.Popen([sys.executable, {str(child_script)!r}, 'child', str(write_fd)], pass_fds=(write_fd,))\n"
                + "os.close(write_fd)\nos.read(read_fd, 64)\n"
                + f"with socket.socket(socket.AF_UNIX) as ready:\n    ready.connect({str(ready_socket)!r})\n    ready.sendall(b'ready')\n",
                encoding="utf-8",
            )
            listener = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            listener.bind(ready_socket.as_posix())
            listener.listen(1)
            listener.settimeout(5.0)
            result: dict[str, int] = {}
            errors: list[Exception] = []
            finished = threading.Event()

            def invoke() -> None:
                try:
                    result["value"] = run_bounded_process(
                        (sys.executable, parent_script.as_posix()),
                        root,
                        {"PATH": os.defpath},
                        root / "stdout",
                        root / "stderr",
                        timeout_seconds=5.0,
                        max_log_bytes=1024,
                    )
                except CandidateProcessError as error:
                    errors.append(error)
                finally:
                    finished.set()

            worker = threading.Thread(target=invoke, daemon=True)
            worker.start()
            try:
                connection = listener.accept()[0]
                with connection:
                    self.assertEqual(connection.recv(64), b"ready")
                self.assertTrue(finished.wait(timeout=5.0))
            finally:
                listener.close()
            if errors:
                raise errors[0]
            self.assertEqual(result.get("value"), 0)
            descendant = int(child_pid.read_text(encoding="utf-8"))
            with self.assertRaises(ProcessLookupError):
                os.kill(descendant, 0)

    def test_normal_exit_without_descendants_is_preserved(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            self.assertEqual(
                run_bounded_process(
                    (sys.executable, "-c", "import sys; sys.exit(0)"),
                    root,
                    {"PATH": os.defpath},
                    root / "stdout",
                    root / "stderr",
                    timeout_seconds=5.0,
                    max_log_bytes=1024,
                ),
                0,
            )

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
