from __future__ import annotations

import os
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from evaluate.multiformat_atomic_publish import (
    READY_BYTES,
    AtomicPublishError,
    AtomicPublishFailure,
    atomic_publish,
)


class WriterFailure(RuntimeError):
    """Raised by a test writer to exercise publication cleanup."""


class MultiFormatAtomicPublishTests(unittest.TestCase):
    def test_complete_tree_is_published_with_ready_last(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            destination = root / "evidence"
            fsync_calls = 0
            real_fsync = os.fsync

            def record_fsync(file_descriptor: int) -> None:
                nonlocal fsync_calls
                fsync_calls += 1
                real_fsync(file_descriptor)

            def write_tree(staging: Path) -> None:
                self.assertEqual(staging.parent, root)
                self.assertTrue(staging.name.startswith(".evidence.stage-"))
                self.assertFalse((staging / "READY").exists())
                nested = staging / "artifacts"
                nested.mkdir()
                (nested / "result.json").write_bytes(b'{"status":"complete"}\n')
                (staging / "manifest.json").write_bytes(b'{"schema_version":1}\n')

            with mock.patch(
                "evaluate.multiformat_atomic_publish.os.fsync",
                side_effect=record_fsync,
            ):
                atomic_publish(destination, write_tree)

            self.assertEqual(
                (destination / "artifacts" / "result.json").read_bytes(),
                b'{"status":"complete"}\n',
            )
            self.assertEqual(READY_BYTES, b"READY\n")
            self.assertEqual((destination / "READY").read_bytes(), b"READY\n")
            self.assertGreaterEqual(fsync_calls, 5)
            self.assertEqual(tuple(root.glob(".evidence.stage-*")), ())

    def test_existing_destination_is_refused_without_calling_writer(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            for destination in (root / "existing-file", root / "existing-directory"):
                with self.subTest(destination=destination.name):
                    if destination.name == "existing-file":
                        destination.write_bytes(b"sentinel")
                    else:
                        destination.mkdir()
                        (destination / "sentinel").write_bytes(b"keep")
                    original = (
                        tuple(
                            (path.relative_to(destination), path.read_bytes())
                            for path in destination.rglob("*")
                            if path.is_file()
                        )
                        if destination.is_dir()
                        else ((Path("."), destination.read_bytes()),)
                    )
                    writer_called = False

                    def write_tree(staging: Path) -> None:
                        nonlocal writer_called
                        writer_called = True
                        (staging / "unexpected").write_bytes(b"unexpected")

                    with self.assertRaises(AtomicPublishError) as raised:
                        atomic_publish(destination, write_tree)

                    self.assertEqual(raised.exception.destination, destination)
                    self.assertIs(
                        raised.exception.failure,
                        AtomicPublishFailure.DESTINATION_EXISTS,
                    )
                    self.assertEqual(
                        raised.exception.failure.value, "destination-exists"
                    )
                    self.assertFalse(writer_called)
                    current = (
                        tuple(
                            (path.relative_to(destination), path.read_bytes())
                            for path in destination.rglob("*")
                            if path.is_file()
                        )
                        if destination.is_dir()
                        else ((Path("."), destination.read_bytes()),)
                    )
                    self.assertEqual(current, original)

    def test_writer_failure_removes_owned_staging_and_never_publishes_ready(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            destination = root / "evidence"

            def fail_writer(staging: Path) -> None:
                (staging / "partial.json").write_bytes(b"partial")
                raise WriterFailure("injected writer failure")

            with self.assertRaises(WriterFailure):
                atomic_publish(destination, fail_writer)

            self.assertFalse(destination.exists())
            self.assertEqual(tuple(root.glob(".evidence.stage-*")), ())

    def test_writer_interruption_removes_owned_staging(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            destination = root / "evidence"

            def interrupt_writer(staging: Path) -> None:
                (staging / "partial.json").write_bytes(b"partial")
                raise KeyboardInterrupt

            with self.assertRaises(KeyboardInterrupt):
                atomic_publish(destination, interrupt_writer)

            self.assertFalse(destination.exists())
            self.assertEqual(tuple(root.glob(".evidence.stage-*")), ())

    def test_ready_write_failure_removes_renamed_tree(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            destination = root / "evidence"

            def write_tree(staging: Path) -> None:
                (staging / "manifest.json").write_bytes(b"complete")

            with (
                mock.patch(
                    "evaluate.multiformat_atomic_publish.os.replace",
                    side_effect=OSError("injected READY replacement failure"),
                ),
                self.assertRaises(OSError),
            ):
                atomic_publish(destination, write_tree)

            self.assertFalse(destination.exists())
            self.assertEqual(tuple(root.glob(".evidence.stage-*")), ())

    def test_writer_cannot_publish_ready_early(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            destination = root / "evidence"

            def write_ready(staging: Path) -> None:
                (staging / "READY").write_bytes(READY_BYTES)

            with self.assertRaises(AtomicPublishError) as raised:
                atomic_publish(destination, write_ready)

            self.assertIs(raised.exception.failure, AtomicPublishFailure.READY_RESERVED)
            self.assertEqual(raised.exception.failure.value, "ready-reserved")
            self.assertFalse(destination.exists())
            self.assertEqual(tuple(root.glob(".evidence.stage-*")), ())


if __name__ == "__main__":
    unittest.main()
