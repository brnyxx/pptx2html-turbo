from __future__ import annotations

import os
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from evaluate import multiformat_snapshot_filesystem as snapshot_filesystem
from evaluate.multiformat_snapshot_publish import publish_snapshot


class PrimaryWriterError(Exception):
    pass


def _write_complete(staging: Path) -> None:
    (staging / "manifest.json").write_bytes(b"complete")


class MultiFormatSnapshotPublishCleanupRaceTests(unittest.TestCase):
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

    def test_lock_replacement_inside_unlink_preserves_attacker_bytes(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            destination = root / "corpus"
            lock_name = ".corpus.snapshot.lock"
            original_unlink = os.unlink
            replaced = False

            def replace_inside_unlink(
                path: str | bytes,
                *,
                dir_fd: int | None = None,
            ) -> None:
                nonlocal replaced
                if not replaced:
                    replaced = True
                    descriptor = os.open(
                        lock_name,
                        os.O_WRONLY | os.O_CREAT | os.O_EXCL,
                        0o600,
                        dir_fd=dir_fd,
                    )
                    try:
                        os.write(descriptor, b"attacker")
                    finally:
                        os.close(descriptor)
                original_unlink(path, dir_fd=dir_fd)

            with mock.patch(
                "evaluate.multiformat_snapshot_publish.os.unlink",
                side_effect=replace_inside_unlink,
            ):
                publish_snapshot(destination, _write_complete)
            self.assertEqual(
                (root / lock_name).read_bytes(),
                b"attacker",
            )

    def test_child_replacement_inside_unlink_preserves_attacker_bytes(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            destination = root / "corpus"
            original_unlink = os.unlink
            replaced = False
            child_path: Path | None = None

            def replace_inside_unlink(
                path: str | bytes,
                *,
                dir_fd: int | None = None,
            ) -> None:
                nonlocal replaced
                if not replaced:
                    replaced = True
                    assert child_path is not None
                    descriptor = os.open(
                        child_path.name,
                        os.O_WRONLY | os.O_CREAT | os.O_EXCL,
                        0o600,
                        dir_fd=dir_fd,
                    )
                    try:
                        os.write(descriptor, b"attacker")
                    finally:
                        os.close(descriptor)
                original_unlink(path, dir_fd=dir_fd)

            def fail(staging: Path) -> None:
                nonlocal child_path
                child_path = staging / "child"
                child_path.write_bytes(b"owned")
                raise PrimaryWriterError("primary")

            with (
                mock.patch(
                    "evaluate.multiformat_snapshot_publish.os.unlink",
                    side_effect=replace_inside_unlink,
                ),
                self.assertRaisesRegex(PrimaryWriterError, "primary"),
            ):
                publish_snapshot(destination, fail)
            stage = next(root.glob(".corpus.stage-*"))
            self.assertEqual((stage / "child").read_bytes(), b"attacker")

    def test_staging_replacement_inside_rmdir_preserves_attacker_directory(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            destination = root / "corpus"
            original_rmdir = os.rmdir
            replaced = False
            staging_path: Path | None = None

            def replace_inside_rmdir(
                path: str | bytes,
                *,
                dir_fd: int | None = None,
            ) -> None:
                nonlocal replaced
                if not replaced:
                    replaced = True
                    assert staging_path is not None
                    os.mkdir(staging_path.name, dir_fd=dir_fd)
                    original_rmdir(path, dir_fd=dir_fd)
                original_rmdir(path, dir_fd=dir_fd)

            def fail(staging: Path) -> None:
                nonlocal staging_path
                staging_path = staging
                raise PrimaryWriterError("primary")

            with (
                mock.patch(
                    "evaluate.multiformat_snapshot_publish.os.rmdir",
                    side_effect=replace_inside_rmdir,
                ),
                self.assertRaisesRegex(PrimaryWriterError, "primary"),
            ):
                publish_snapshot(destination, fail)
            self.assertEqual(len(tuple(root.glob(".corpus.stage-*"))), 1)

    def test_child_directory_replacement_inside_rmdir_is_preserved(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            destination = root / "corpus"
            original_rmdir = os.rmdir
            replaced = False
            child_path: Path | None = None

            def replace_inside_rmdir(
                path: str | bytes,
                *,
                dir_fd: int | None = None,
            ) -> None:
                nonlocal replaced
                if not replaced:
                    replaced = True
                    assert child_path is not None
                    os.mkdir(child_path.name, dir_fd=dir_fd)
                    original_rmdir(path, dir_fd=dir_fd)
                original_rmdir(path, dir_fd=dir_fd)

            def fail(staging: Path) -> None:
                nonlocal child_path
                child_path = staging / "childdir"
                child_path.mkdir()
                raise PrimaryWriterError("primary")

            with (
                mock.patch(
                    "evaluate.multiformat_snapshot_filesystem.os.rmdir",
                    side_effect=replace_inside_rmdir,
                ),
                self.assertRaisesRegex(PrimaryWriterError, "primary"),
            ):
                publish_snapshot(destination, fail)
            stage = next(root.glob(".corpus.stage-*"))
            self.assertTrue((stage / "childdir").is_dir())


if __name__ == "__main__":
    unittest.main()
