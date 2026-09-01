from __future__ import annotations

import io
import os
import subprocess
import tempfile
import threading
import unittest
from pathlib import Path
from unittest.mock import patch

from evaluate.multiformat_candidate_process import (
    CandidateProcessError,
    CandidateProcessFailure,
    run_bounded_process,
)
from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_native_unit_files import stable_file
from evaluate.multiformat_native_unit_process import run_native_process
from evaluate.multiformat_native_unit_types import (
    NativeProcessRequest,
    NativeUnitFailure,
)
from evaluate.tests.multiformat_native_unit_fixture import make_native_unit_fixture


class MultiFormatNativeUnitTerminalTests(unittest.TestCase):
    def test_tool_substitution_before_popen_cannot_execute_outside_path(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_unit_fixture(root)
            trusted = root / "trusted-tool"
            outside = root / "outside-tool"
            marker = root / "marker"
            outside_marker = root / "outside-marker"
            _ = trusted.write_text(
                '#!/bin/sh\nprintf trusted > "$1"\n', encoding="utf-8"
            )
            _ = outside.write_text(
                f"#!/bin/sh\nprintf outside > {str(outside_marker)!r}\n",
                encoding="utf-8",
            )
            _ = trusted.chmod(0o755)
            _ = outside.chmod(0o755)
            expected = stable_file(
                trusted,
                fixture.request(root, DocumentFormat.PDF),
                NativeUnitFailure.TOOL_MISSING,
            )
            request = NativeProcessRequest(
                (trusted.as_posix(), marker.as_posix()),
                root,
                (("PATH", os.defpath),),
                root / "stdout",
                root / "stderr",
                5,
                1024,
                expected,
            )

            def race_before_popen() -> None:
                _ = trusted.unlink()
                _ = trusted.symlink_to(outside)

            with patch(
                "evaluate.multiformat_candidate_process._before_popen",
                side_effect=race_before_popen,
            ):
                _ = run_native_process(request)

            self.assertEqual(marker.read_text(encoding="utf-8"), "trusted")
            self.assertFalse(outside_marker.exists())

    def test_wait_oserror_kills_and_reaps_with_bounded_waits(self) -> None:
        class WaitFailureProcess:
            pid: int = 123
            stdout: io.BytesIO = io.BytesIO()
            stderr: io.BytesIO = io.BytesIO()
            waits: list[float | None]

            def __init__(self) -> None:
                self.waits = []

            def wait(self, timeout: float | None = None) -> int:
                self.waits.append(timeout)
                if len(self.waits) == 1:
                    raise OSError("wait failed")
                return 0

            def poll(self) -> None:
                return None

        process = WaitFailureProcess()
        with (
            patch.object(subprocess, "Popen", return_value=process),
            patch(
                "evaluate.multiformat_candidate_process._kill_process_group",
                return_value=True,
            ) as kill,
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

        self.assertIs(
            raised.exception.failure, CandidateProcessFailure.TERMINATION_FAILED
        )
        kill.assert_called_once()
        self.assertEqual(len(process.waits), 2)
        self.assertTrue(all(value is not None for value in process.waits))

    def test_failed_group_kill_does_not_claim_parent_kill_is_group_success(
        self,
    ) -> None:
        class Process:
            pid: int = 123
            stdout: io.BytesIO = io.BytesIO()
            stderr: io.BytesIO = io.BytesIO()
            killed: bool = False
            waits: int

            def __init__(self) -> None:
                self.waits = 0

            def kill(self) -> None:
                self.killed = True

            def wait(self, timeout: float | None = None) -> int:
                if timeout is None:
                    raise AssertionError("unbounded wait")
                self.waits += 1
                if self.waits == 1:
                    raise subprocess.TimeoutExpired(("fake",), timeout)
                return 0

            def poll(self) -> None:
                return None

        process = Process()
        with (
            patch.object(subprocess, "Popen", return_value=process),
            patch(
                "evaluate.multiformat_candidate_process.os.killpg",
                side_effect=ProcessLookupError,
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
        self.assertTrue(process.killed)

    def test_active_readers_are_interrupted_at_fd_level_and_joined(self) -> None:
        stdout_read, stdout_write = os.pipe()
        stderr_read, stderr_write = os.pipe()
        stdout = os.fdopen(stdout_read, "rb")
        stderr = os.fdopen(stderr_read, "rb")

        class Process:
            pid: int = 123

            def __init__(
                self, stdout: io.BufferedReader, stderr: io.BufferedReader
            ) -> None:
                self.stdout: io.BufferedReader = stdout
                self.stderr: io.BufferedReader = stderr

            def wait(self, timeout: float | None = None) -> int:
                _ = timeout
                return 0

            def poll(self) -> int:
                return 0

        process = Process(stdout, stderr)
        finished = threading.Event()
        results: list[int] = []
        errors: list[CandidateProcessError] = []

        def invoke() -> None:
            try:
                with tempfile.TemporaryDirectory() as temp_dir:
                    root = Path(temp_dir)
                    results.append(
                        run_bounded_process(
                            ("fake",),
                            root,
                            {},
                            root / "stdout",
                            root / "stderr",
                            timeout_seconds=1,
                            max_log_bytes=1024,
                        )
                    )
            except CandidateProcessError as error:
                errors.append(error)
            finally:
                finished.set()

        try:
            with (
                patch.object(subprocess, "Popen", return_value=process),
                patch(
                    "evaluate.multiformat_candidate_process._kill_process_group",
                    return_value=True,
                ),
            ):
                worker = threading.Thread(target=invoke, daemon=True)
                worker.start()
                self.assertTrue(finished.wait(timeout=4.0))
        finally:
            os.close(stdout_write)
            os.close(stderr_write)

        self.assertEqual(results, [0])
        self.assertEqual(errors, [])


if __name__ == "__main__":
    _ = unittest.main()
