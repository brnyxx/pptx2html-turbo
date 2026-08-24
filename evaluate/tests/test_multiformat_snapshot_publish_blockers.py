from __future__ import annotations

import os
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from evaluate import multiformat_snapshot_filesystem as snapshot_filesystem
from evaluate import multiformat_snapshot_publish as snapshot_publish
from evaluate.multiformat_snapshot_publish import (
    SnapshotPublishError,
    publish_snapshot,
)


class PrimaryWriterError(Exception):
    pass


def _write_complete(staging: Path) -> None:
    (staging / "manifest.json").write_bytes(b"complete")


class MultiFormatSnapshotPublishBlockerTests(unittest.TestCase):
    def test_restore_race_preserves_attacker_and_owned_tombstone(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            destination = root / "corpus"
            original_atomic = snapshot_filesystem.atomic_rename_noreplace
            original_unlink = snapshot_filesystem.os.unlink
            attacker_inserted = False
            forced_failure = False

            def insert_attacker(
                staging: Path,
                target: Path,
                parent_descriptor: int,
            ) -> None:
                nonlocal attacker_inserted
                if target.name == "child" and not attacker_inserted:
                    attacker_inserted = True
                    descriptor = os.open(
                        target.name,
                        os.O_WRONLY | os.O_CREAT | os.O_EXCL,
                        0o600,
                        dir_fd=parent_descriptor,
                    )
                    try:
                        os.write(descriptor, b"attacker")
                    finally:
                        os.close(descriptor)
                original_atomic(staging, target, parent_descriptor)

            def fail_first_unlink(
                path: str | os.PathLike[str],
                *,
                dir_fd: int | None = None,
            ) -> None:
                nonlocal forced_failure
                if not forced_failure and Path(path).name.startswith(".cleanup-"):
                    forced_failure = True
                    raise OSError("forced cleanup failure")
                original_unlink(path, dir_fd=dir_fd)

            def fail_writer(staging: Path) -> None:
                (staging / "child").write_bytes(b"owned")
                raise PrimaryWriterError("primary")

            with (
                mock.patch.object(
                    snapshot_filesystem,
                    "atomic_rename_noreplace",
                    side_effect=insert_attacker,
                ),
                mock.patch.object(
                    snapshot_filesystem.os,
                    "unlink",
                    side_effect=fail_first_unlink,
                ),
                self.assertRaisesRegex(PrimaryWriterError, "primary"),
            ):
                publish_snapshot(destination, fail_writer)

            stage = next(root.glob(".corpus.stage-*"))
            self.assertEqual((stage / "child").read_bytes(), b"attacker")
            self.assertEqual(
                next(stage.glob(".cleanup-*")).read_bytes(),
                b"owned",
            )

    def test_lock_file_contention_closes_parent_once(self) -> None:
        self._assert_lock_failure(lock_kind="file")

    def test_lock_directory_contention_closes_parent_once(self) -> None:
        self._assert_lock_failure(lock_kind="directory")

    def test_lock_acquire_oserror_closes_parent_once(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            destination = root / "corpus"
            original_open = snapshot_publish.os.open
            original_close = snapshot_publish.os.close
            opened: list[int] = []
            closed: list[int] = []

            def fail_lock_open(
                path: str | os.PathLike[str],
                flags: int,
                mode: int = 0o777,
                *,
                dir_fd: int | None = None,
            ) -> int:
                if Path(path).name == ".corpus.snapshot.lock":
                    raise OSError("lock acquire failure")
                descriptor = original_open(path, flags, mode, dir_fd=dir_fd)
                opened.append(descriptor)
                return descriptor

            def track_close(descriptor: int) -> None:
                closed.append(descriptor)
                original_close(descriptor)

            with (
                mock.patch.object(
                    snapshot_publish.os, "open", side_effect=fail_lock_open
                ),
                mock.patch.object(
                    snapshot_publish.os, "close", side_effect=track_close
                ),
                self.assertRaises(SnapshotPublishError),
            ):
                publish_snapshot(destination, _write_complete)

            self.assertEqual(closed, opened)

    def _assert_lock_failure(self, *, lock_kind: str) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            destination = root / "corpus"
            lock = root / ".corpus.snapshot.lock"
            if lock_kind == "file":
                lock.write_bytes(b"foreign")
            else:
                lock.mkdir()
            original_open = snapshot_publish.os.open
            original_close = snapshot_publish.os.close
            opened: list[int] = []
            closed: list[int] = []

            def track_open(
                path: str | os.PathLike[str],
                flags: int,
                mode: int = 0o777,
                *,
                dir_fd: int | None = None,
            ) -> int:
                descriptor = original_open(path, flags, mode, dir_fd=dir_fd)
                opened.append(descriptor)
                return descriptor

            def track_close(descriptor: int) -> None:
                closed.append(descriptor)
                original_close(descriptor)

            with (
                mock.patch.object(snapshot_publish.os, "open", side_effect=track_open),
                mock.patch.object(
                    snapshot_publish.os, "close", side_effect=track_close
                ),
                self.assertRaises(SnapshotPublishError),
            ):
                publish_snapshot(destination, _write_complete)

            self.assertEqual(closed, opened)


if __name__ == "__main__":
    unittest.main()
