from __future__ import annotations

import io
import os
import subprocess
import tempfile
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
from evaluate.multiformat_native_unit_trusted import open_trusted_executable
from evaluate.multiformat_native_unit_types import (
    NativeProcessRequest,
    NativeUnitFailure,
)
from evaluate.tests.multiformat_native_unit_fixture import make_native_unit_fixture


class MultiFormatNativeUnitFinal2BlockerTests(unittest.TestCase):
    def test_binary_success_closes_trusted_source_descriptor(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_unit_fixture(root)
            executable = Path("/bin/echo")
            expected = stable_file(
                executable,
                fixture.request(root, DocumentFormat.PDF),
                NativeUnitFailure.TOOL_MISSING,
            )
            request = NativeProcessRequest(
                (executable.as_posix(), "descriptor-closed"),
                root,
                (("PATH", os.defpath),),
                root / "stdout",
                root / "stderr",
                5,
                1024,
                expected,
            )
            descriptors: list[int] = []

            def remember_descriptor(path: Path, identity):
                trusted = open_trusted_executable(path, identity)
                descriptors.append(trusted.descriptor)
                return trusted

            with patch(
                "evaluate.multiformat_native_unit_process.open_trusted_executable",
                side_effect=remember_descriptor,
            ):
                self.assertEqual(run_native_process(request), 0)

            self.assertEqual(len(descriptors), 1)
            with self.assertRaises(OSError):
                _ = os.fstat(descriptors[0])

    def test_real_root_owned_echo_uses_private_snapshot_directory(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_unit_fixture(root)
            request_context = fixture.request(root, DocumentFormat.PDF)
            executable = Path("/bin/echo")
            expected = stable_file(
                executable,
                request_context,
                NativeUnitFailure.TOOL_MISSING,
            )
            request = NativeProcessRequest(
                (executable.as_posix(), "trusted-echo"),
                root,
                (("PATH", os.defpath),),
                root / "stdout",
                root / "stderr",
                5,
                1024,
                expected,
            )

            self.assertEqual(run_native_process(request), 0)
            self.assertEqual((root / "stdout").read_bytes(), b"trusted-echo\n")

    def test_snapshot_substitution_at_popen_cannot_execute_outside(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_unit_fixture(root)
            executable = Path("/bin/echo")
            outside = root / "outside-tool"
            outside_marker = root / "outside-marker"
            _ = outside.write_text(
                f"#!/bin/sh\nprintf outside > {str(outside_marker)!r}\n",
                encoding="utf-8",
            )
            _ = outside.chmod(0o755)
            expected = stable_file(
                executable,
                fixture.request(root, DocumentFormat.PDF),
                NativeUnitFailure.TOOL_MISSING,
            )
            request = NativeProcessRequest(
                (executable.as_posix(), "trusted-binary"),
                root,
                (("PATH", os.defpath),),
                root / "stdout",
                root / "stderr",
                5,
                1024,
                expected,
            )

            blocked = False

            def race_before_popen() -> None:
                nonlocal blocked
                snapshots = tuple(root.rglob(".trusted-executable-*"))
                self.assertEqual(len(snapshots), 1)
                snapshot = snapshots[0]
                try:
                    snapshot.unlink()
                except PermissionError:
                    blocked = True
                    return
                _ = snapshot.symlink_to(outside)

            with patch(
                "evaluate.multiformat_candidate_process._before_popen",
                side_effect=race_before_popen,
            ):
                self.assertEqual(run_native_process(request), 0)

            self.assertTrue(blocked)
            self.assertFalse(outside_marker.exists())

    def test_unknown_group_state_still_boundedly_reaps_parent(self) -> None:
        class Process:
            pid: int = 123
            stdout: io.BytesIO = io.BytesIO()
            stderr: io.BytesIO = io.BytesIO()

            def __init__(self) -> None:
                self.waits: list[float | None] = []

            def wait(self, timeout: float | None = None) -> int:
                self.waits.append(timeout)
                if len(self.waits) == 1:
                    timeout_value = 0.0 if timeout is None else timeout
                    raise subprocess.TimeoutExpired(("fake",), timeout_value)
                return 0

            def kill(self) -> None:
                return

            def poll(self) -> int | None:
                return None

        process = Process()
        with (
            patch.object(subprocess, "Popen", return_value=process),
            patch(
                "evaluate.multiformat_candidate_process.os.killpg",
                side_effect=PermissionError,
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
            raised.exception.failure,
            CandidateProcessFailure.TERMINATION_FAILED,
        )
        self.assertEqual(len(process.waits), 2)
        self.assertTrue(all(value is not None for value in process.waits))

    def test_reaped_parent_and_absent_group_can_report_success(self) -> None:
        class Process:
            pid: int = 123
            stdout: io.BytesIO = io.BytesIO()
            stderr: io.BytesIO = io.BytesIO()

            def kill(self) -> None:
                return

            def wait(self, timeout: float | None = None) -> int:
                _ = timeout
                return 0

            def poll(self) -> int | None:
                return 0

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
            self.assertEqual(
                run_bounded_process(
                    ("fake",),
                    root,
                    {},
                    root / "stdout",
                    root / "stderr",
                    timeout_seconds=1,
                    max_log_bytes=1024,
                ),
                0,
            )

    def test_parent_reaped_does_not_prove_descendant_group_absence(self) -> None:
        class Process:
            pid: int = 123
            stdout: io.BytesIO = io.BytesIO()
            stderr: io.BytesIO = io.BytesIO()

            def kill(self) -> None:
                return

            def wait(self, timeout: float | None = None) -> int:
                _ = timeout
                return 0

            def poll(self) -> int | None:
                return 0

        process = Process()
        with (
            patch.object(subprocess, "Popen", return_value=process),
            patch(
                "evaluate.multiformat_candidate_process.os.killpg",
                side_effect=PermissionError,
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

        self.assertIs(
            raised.exception.failure,
            CandidateProcessFailure.TERMINATION_FAILED,
        )


if __name__ == "__main__":
    _ = unittest.main()
