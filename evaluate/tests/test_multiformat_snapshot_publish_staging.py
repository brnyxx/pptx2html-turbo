from __future__ import annotations

import errno
import os
import tempfile
import unittest
from collections.abc import Callable
from pathlib import Path
from unittest import mock

from evaluate import multiformat_snapshot_filesystem as snapshot_filesystem
from evaluate import multiformat_snapshot_publish as snapshot_publish
from evaluate.multiformat_snapshot_publish import (
    SnapshotPublishError,
    SnapshotPublishFailure,
    publish_snapshot,
)


def _write_complete(staging: Path) -> None:
    (staging / "manifest.json").write_bytes(b"complete")


def _fd_count() -> int:
    return len(os.listdir("/dev/fd"))


class MultiFormatSnapshotPublishStagingTests(unittest.TestCase):
    def test_staging_open_failure_is_typed_and_cleans_empty_stage(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            before = _fd_count()
            root = Path(temp_dir)
            destination = root / "corpus"
            original_mkdtemp = snapshot_publish.tempfile.mkdtemp
            original_open = snapshot_publish.os.open
            original_close = snapshot_publish.os.close
            original_fstat = snapshot_publish.os.fstat
            created: Path | None = None
            opened: list[int] = []
            closed: list[int] = []

            def record_stage(*, prefix: str, dir: str) -> str:
                nonlocal created
                path = original_mkdtemp(prefix=prefix, dir=dir)
                created = Path(path)
                return path

            def fail_stage_open(
                path: str | os.PathLike[str],
                flags: int,
                mode: int = 0o777,
                *,
                dir_fd: int | None = None,
            ) -> int:
                if created is not None and Path(path) == created:
                    raise OSError("staging open failure")
                descriptor = original_open(path, flags, mode, dir_fd=dir_fd)
                opened.append(descriptor)
                return descriptor

            def track_close(descriptor: int) -> None:
                closed.append(descriptor)
                original_close(descriptor)

            with (
                mock.patch.object(
                    snapshot_publish.tempfile, "mkdtemp", side_effect=record_stage
                ),
                mock.patch.object(
                    snapshot_publish.os, "open", side_effect=fail_stage_open
                ),
                mock.patch.object(
                    snapshot_publish.os, "close", side_effect=track_close
                ),
                self.assertRaises(SnapshotPublishError) as raised,
            ):
                publish_snapshot(destination, _write_complete)

            self.assertIs(
                raised.exception.failure,
                SnapshotPublishFailure.PUBLICATION_FAILED,
            )
            self.assertIsNotNone(created)
            assert created is not None
            self.assertFalse(created.exists())
            self.assertEqual(tuple(root.glob(".corpus.stage-*")), ())
            self.assertFalse((root / ".corpus.snapshot.lock").exists())
            self.assertCountEqual(closed, opened)
            self._assert_descriptors_closed(opened, original_fstat, original_close)
            self.assertEqual(_fd_count(), before)

    def test_staging_substitution_is_preserved_with_cleanup_note(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            before = _fd_count()
            root = Path(temp_dir)
            destination = root / "corpus"
            original_mkdtemp = snapshot_publish.tempfile.mkdtemp
            original_open = snapshot_publish.os.open
            original_close = snapshot_publish.os.close
            original_fstat = snapshot_publish.os.fstat
            original_rmdir = snapshot_filesystem.os.rmdir
            created: Path | None = None
            opened: list[int] = []
            closed: list[int] = []
            replaced = False

            def record_stage(*, prefix: str, dir: str) -> str:
                nonlocal created
                path = original_mkdtemp(prefix=prefix, dir=dir)
                created = Path(path)
                return path

            def fail_stage_open(
                path: str | os.PathLike[str],
                flags: int,
                mode: int = 0o777,
                *,
                dir_fd: int | None = None,
            ) -> int:
                if created is not None and Path(path) == created:
                    raise OSError("staging open failure")
                descriptor = original_open(path, flags, mode, dir_fd=dir_fd)
                opened.append(descriptor)
                return descriptor

            def replace_before_rmdir(
                path: str | os.PathLike[str],
                *,
                dir_fd: int | None = None,
            ) -> None:
                nonlocal replaced
                if not replaced:
                    replaced = True
                    assert created is not None
                    if created.exists():
                        created.rename(root / "owned-stage")
                    os.mkdir(created.name, dir_fd=dir_fd)
                    (created / "sentinel").write_bytes(b"attacker")
                original_rmdir(path, dir_fd=dir_fd)

            def track_close(descriptor: int) -> None:
                closed.append(descriptor)
                original_close(descriptor)

            with (
                mock.patch.object(
                    snapshot_publish.tempfile, "mkdtemp", side_effect=record_stage
                ),
                mock.patch.object(
                    snapshot_publish.os, "open", side_effect=fail_stage_open
                ),
                mock.patch.object(
                    snapshot_filesystem.os, "rmdir", side_effect=replace_before_rmdir
                ),
                mock.patch.object(
                    snapshot_publish.os, "close", side_effect=track_close
                ),
                self.assertRaises(SnapshotPublishError) as raised,
            ):
                publish_snapshot(destination, _write_complete)

            self.assertIs(
                raised.exception.failure,
                SnapshotPublishFailure.PUBLICATION_FAILED,
            )
            self.assertIsNotNone(created)
            assert created is not None
            self.assertEqual((created / "sentinel").read_bytes(), b"attacker")
            self.assertCountEqual(closed, opened)
            self._assert_descriptors_closed(opened, original_fstat, original_close)
            self.assertEqual(_fd_count(), before)

    def test_empty_staging_substitution_survives_rmdir_boundary(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            destination = root / "corpus"
            original_mkdtemp = snapshot_publish.tempfile.mkdtemp
            original_open = snapshot_publish.os.open
            original_rmdir = snapshot_filesystem.os.rmdir
            created: Path | None = None
            owned_identity: tuple[int, int] | None = None
            replaced = False

            def record_stage(*, prefix: str, dir: str) -> str:
                nonlocal created, owned_identity
                path = original_mkdtemp(prefix=prefix, dir=dir)
                created = Path(path)
                information = created.stat()
                owned_identity = (information.st_dev, information.st_ino)
                return path

            def fail_stage_open(
                path: str | os.PathLike[str],
                flags: int,
                mode: int = 0o777,
                *,
                dir_fd: int | None = None,
            ) -> int:
                if created is not None and Path(path) == created:
                    raise OSError("staging open failure")
                return original_open(path, flags, mode, dir_fd=dir_fd)

            def replace_before_rmdir(
                path: str | os.PathLike[str],
                *,
                dir_fd: int | None = None,
            ) -> None:
                nonlocal replaced
                if not replaced:
                    replaced = True
                    assert created is not None
                    if created.exists():
                        created.rename(root / "owned-stage")
                    os.mkdir(created.name, dir_fd=dir_fd)
                original_rmdir(path, dir_fd=dir_fd)

            with (
                mock.patch.object(
                    snapshot_publish.tempfile, "mkdtemp", side_effect=record_stage
                ),
                mock.patch.object(
                    snapshot_publish.os, "open", side_effect=fail_stage_open
                ),
                mock.patch.object(
                    snapshot_filesystem.os, "rmdir", side_effect=replace_before_rmdir
                ),
                self.assertRaises(SnapshotPublishError),
            ):
                publish_snapshot(destination, _write_complete)

            self.assertIsNotNone(created)
            self.assertIsNotNone(owned_identity)
            assert created is not None
            assert owned_identity is not None
            information = created.stat()
            self.assertNotEqual(
                (information.st_dev, information.st_ino),
                owned_identity,
            )
            self.assertFalse((root / ".corpus.snapshot.lock").exists())

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


if __name__ == "__main__":
    unittest.main()
