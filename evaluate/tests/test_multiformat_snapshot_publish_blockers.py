from __future__ import annotations

import errno
import os
import tempfile
import unittest
from collections.abc import Callable
from pathlib import Path
from unittest import mock

from evaluate import multiformat_snapshot_publish as snapshot_publish
from evaluate.multiformat_snapshot_publish import (
    SnapshotPublishError,
    publish_snapshot,
)


def _write_complete(staging: Path) -> None:
    (staging / "manifest.json").write_bytes(b"complete")


def _fd_count() -> int:
    return len(os.listdir("/dev/fd"))


class MultiFormatSnapshotPublishBlockerTests(unittest.TestCase):
    def test_lock_file_contention_closes_parent_once(self) -> None:
        self._assert_lock_failure(lock_kind="file")

    def test_lock_directory_contention_closes_parent_once(self) -> None:
        self._assert_lock_failure(lock_kind="directory")

    def test_lock_acquire_oserror_closes_parent_once(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            before = _fd_count()
            root = Path(temp_dir)
            destination = root / "corpus"
            original_open = snapshot_publish.os.open
            original_close = snapshot_publish.os.close
            original_fstat = snapshot_publish.os.fstat
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

            try:
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
                self.assertCountEqual(closed, opened)
                self._assert_descriptors_closed(opened, original_fstat, original_close)
                self.assertEqual(_fd_count(), before)
            finally:
                self._close_valid_descriptors(opened, original_fstat, original_close)

    def test_lock_post_open_fstat_failure_closes_both_descriptors(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            before = _fd_count()
            root = Path(temp_dir)
            destination = root / "corpus"
            original_open = snapshot_publish.os.open
            original_close = snapshot_publish.os.close
            original_dup = snapshot_publish.os.dup
            original_fstat = snapshot_publish.os.fstat
            opened: list[int] = []
            closed: list[int] = []
            lock_descriptor: int | None = None
            failed = False

            def track_open(
                path: str | os.PathLike[str],
                flags: int,
                mode: int = 0o777,
                *,
                dir_fd: int | None = None,
            ) -> int:
                nonlocal lock_descriptor
                descriptor = original_open(path, flags, mode, dir_fd=dir_fd)
                opened.append(descriptor)
                if Path(path).name == ".corpus.snapshot.lock":
                    lock_descriptor = descriptor
                return descriptor

            def track_dup(descriptor: int) -> int:
                duplicated = original_dup(descriptor)
                opened.append(duplicated)
                return duplicated

            def fail_lock_fstat(descriptor: int) -> os.stat_result:
                nonlocal failed
                if descriptor == lock_descriptor and not failed:
                    failed = True
                    raise OSError("lock identity failure")
                return original_fstat(descriptor)

            def track_close(descriptor: int) -> None:
                closed.append(descriptor)
                original_close(descriptor)

            try:
                with (
                    mock.patch.object(
                        snapshot_publish.os, "open", side_effect=track_open
                    ),
                    mock.patch.object(
                        snapshot_publish.os, "fstat", side_effect=fail_lock_fstat
                    ),
                    mock.patch.object(
                        snapshot_publish.os, "dup", side_effect=track_dup
                    ),
                    mock.patch.object(
                        snapshot_publish.os, "close", side_effect=track_close
                    ),
                    self.assertRaises(SnapshotPublishError),
                ):
                    publish_snapshot(destination, _write_complete)
                self.assertCountEqual(closed, opened)
                self._assert_descriptors_closed(opened, original_fstat, original_close)
            finally:
                self._close_valid_descriptors(opened, original_fstat, original_close)
            publish_snapshot(destination, _write_complete)
            self.assertEqual(_fd_count(), before)

    def test_lock_recovery_preserves_attacker_replacement(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            before = _fd_count()
            root = Path(temp_dir)
            destination = root / "corpus"
            lock = root / ".corpus.snapshot.lock"
            original_open = snapshot_publish.os.open
            original_fstat = snapshot_publish.os.fstat
            lock_descriptor: int | None = None
            replaced = False

            def track_open(
                path: str | os.PathLike[str],
                flags: int,
                mode: int = 0o777,
                *,
                dir_fd: int | None = None,
            ) -> int:
                nonlocal lock_descriptor
                descriptor = original_open(path, flags, mode, dir_fd=dir_fd)
                if Path(path).name == lock.name:
                    lock_descriptor = descriptor
                return descriptor

            def replace_then_fail(descriptor: int) -> os.stat_result:
                nonlocal replaced
                information = original_fstat(descriptor)
                if descriptor == lock_descriptor and not replaced:
                    replaced = True
                    lock.rename(root / "owned-lock")
                    lock.write_bytes(b"attacker")
                    raise OSError("lock identity failure")
                return information

            with (
                mock.patch.object(snapshot_publish.os, "open", side_effect=track_open),
                mock.patch.object(
                    snapshot_publish.os, "fstat", side_effect=replace_then_fail
                ),
                self.assertRaises(SnapshotPublishError) as raised,
            ):
                publish_snapshot(destination, _write_complete)

            cause = raised.exception.__cause__
            self.assertIsNotNone(cause)
            assert cause is not None
            self.assertTrue(any("cleanup failed" in note for note in cause.__notes__))
            self.assertEqual(lock.read_bytes(), b"attacker")
            self.assertEqual(_fd_count(), before)

    def _assert_lock_failure(self, *, lock_kind: str) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            before = _fd_count()
            root = Path(temp_dir)
            destination = root / "corpus"
            lock = root / ".corpus.snapshot.lock"
            if lock_kind == "file":
                lock.write_bytes(b"foreign")
            else:
                lock.mkdir()
            original_open = snapshot_publish.os.open
            original_close = snapshot_publish.os.close
            original_fstat = snapshot_publish.os.fstat
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

            try:
                with (
                    mock.patch.object(
                        snapshot_publish.os, "open", side_effect=track_open
                    ),
                    mock.patch.object(
                        snapshot_publish.os, "close", side_effect=track_close
                    ),
                    self.assertRaises(SnapshotPublishError),
                ):
                    publish_snapshot(destination, _write_complete)
                self.assertCountEqual(closed, opened)
                self._assert_descriptors_closed(opened, original_fstat, original_close)
                self.assertEqual(_fd_count(), before)
            finally:
                self._close_valid_descriptors(opened, original_fstat, original_close)

    def _assert_descriptors_closed(
        self,
        descriptors: list[int],
        original_fstat: Callable[[int], os.stat_result],
        original_close: Callable[[int], None],
    ) -> None:
        for descriptor in descriptors:
            try:
                original_fstat(descriptor)
            except OSError as error:
                self.assertEqual(error.errno, errno.EBADF)
            else:
                self.fail(f"descriptor {descriptor} remained open")

    def _close_valid_descriptors(
        self,
        descriptors: list[int],
        original_fstat: Callable[[int], os.stat_result],
        original_close: Callable[[int], None],
    ) -> None:
        for descriptor in descriptors:
            try:
                original_fstat(descriptor)
            except OSError:
                continue
            original_close(descriptor)


if __name__ == "__main__":
    unittest.main()
