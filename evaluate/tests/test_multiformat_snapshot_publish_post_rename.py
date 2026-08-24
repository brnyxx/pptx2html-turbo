from __future__ import annotations

import os
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from evaluate import multiformat_snapshot_publish as snapshot_publish
from evaluate.multiformat_snapshot_publish import (
    SnapshotPublishError,
    SnapshotPublishFailure,
    publish_snapshot,
)


def _write_complete(staging: Path) -> None:
    (staging / "manifest.json").write_bytes(b"complete")


class MultiFormatSnapshotPublishPostRenameTests(unittest.TestCase):
    def test_target_open_failure_rolls_back_owned_destination(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            destination = root / "corpus"
            original_open = snapshot_publish.os.open

            def fail_target_open(
                path: str | os.PathLike[str],
                flags: int,
                mode: int = 0o777,
                *,
                dir_fd: int | None = None,
            ) -> int:
                if Path(path) == destination.resolve() and flags & os.O_DIRECTORY:
                    raise OSError("target open failure")
                return original_open(path, flags, mode, dir_fd=dir_fd)

            with (
                mock.patch.object(
                    snapshot_publish.os, "open", side_effect=fail_target_open
                ),
                self.assertRaises(SnapshotPublishError) as raised,
            ):
                publish_snapshot(destination, _write_complete)

            self.assertIs(
                raised.exception.failure,
                SnapshotPublishFailure.PUBLICATION_FAILED,
            )
            self.assertFalse(destination.exists())

    def test_target_fstat_failure_rolls_back_owned_destination(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            destination = root / "corpus"
            original_open = snapshot_publish.os.open
            original_fstat = snapshot_publish.os.fstat
            target_descriptor: int | None = None
            failed = False

            def capture_target_open(
                path: str | os.PathLike[str],
                flags: int,
                mode: int = 0o777,
                *,
                dir_fd: int | None = None,
            ) -> int:
                nonlocal target_descriptor
                descriptor = original_open(path, flags, mode, dir_fd=dir_fd)
                if Path(path) == destination.resolve() and flags & os.O_DIRECTORY:
                    target_descriptor = descriptor
                return descriptor

            def fail_target_fstat(descriptor: int) -> os.stat_result:
                nonlocal failed
                if descriptor == target_descriptor and not failed:
                    failed = True
                    raise OSError("target fstat failure")
                return original_fstat(descriptor)

            with (
                mock.patch.object(
                    snapshot_publish.os, "open", side_effect=capture_target_open
                ),
                mock.patch.object(
                    snapshot_publish.os, "fstat", side_effect=fail_target_fstat
                ),
                self.assertRaises(SnapshotPublishError) as raised,
            ):
                publish_snapshot(destination, _write_complete)

            self.assertIs(
                raised.exception.failure,
                SnapshotPublishFailure.PUBLICATION_FAILED,
            )
            self.assertFalse(destination.exists())

    def test_target_identity_failure_preserves_attacker_destination(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            destination = root / "corpus"
            original_open = snapshot_publish.os.open
            original_fstat = snapshot_publish.os.fstat
            target_descriptor: int | None = None
            replaced = False

            def capture_target_open(
                path: str | os.PathLike[str],
                flags: int,
                mode: int = 0o777,
                *,
                dir_fd: int | None = None,
            ) -> int:
                nonlocal target_descriptor
                descriptor = original_open(path, flags, mode, dir_fd=dir_fd)
                if Path(path) == destination.resolve() and flags & os.O_DIRECTORY:
                    target_descriptor = descriptor
                return descriptor

            def fail_identity(descriptor: int) -> os.stat_result:
                nonlocal replaced
                information = original_fstat(descriptor)
                if descriptor == target_descriptor and not replaced:
                    replaced = True
                    destination.rename(root / "owned")
                    destination.mkdir()
                    (destination / "sentinel").write_bytes(b"attacker")
                    values = list(information)
                    values[1] += 1
                    return os.stat_result(values)
                return information

            with (
                mock.patch.object(
                    snapshot_publish.os, "open", side_effect=capture_target_open
                ),
                mock.patch.object(
                    snapshot_publish.os, "fstat", side_effect=fail_identity
                ),
                self.assertRaises(SnapshotPublishError) as raised,
            ):
                publish_snapshot(destination, _write_complete)

            self.assertIs(
                raised.exception.failure,
                SnapshotPublishFailure.PUBLICATION_FAILED,
            )
            self.assertEqual((destination / "sentinel").read_bytes(), b"attacker")


if __name__ == "__main__":
    unittest.main()
