from __future__ import annotations

import errno
import os
import shutil
import sys
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


class PrimaryWriterError(Exception):
    pass


def _write_complete(staging: Path) -> None:
    (staging / "manifest.json").write_bytes(b"complete")


class MultiFormatSnapshotPublishRaceTests(unittest.TestCase):
    def test_platform_destination_error_maps_to_typed_failure(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            target = Path(temp_dir) / "corpus"
            with (
                mock.patch(
                    "evaluate.multiformat_snapshot_publish.atomic_rename_noreplace",
                    side_effect=FileExistsError(errno.EEXIST, "exists"),
                ),
                self.assertRaises(SnapshotPublishError) as raised,
            ):
                snapshot_publish._atomic_rename_noreplace(
                    target.with_name("staging"), target, -1
                )
            self.assertIs(
                raised.exception.failure,
                SnapshotPublishFailure.DESTINATION_EXISTS,
            )

    def test_unsupported_platform_fails_typed_at_filesystem_boundary(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            target = Path(temp_dir) / "corpus"
            with (
                mock.patch.object(snapshot_filesystem.sys, "platform", "unsupported"),
                self.assertRaises(OSError) as raised,
            ):
                snapshot_filesystem.atomic_rename_noreplace(
                    target.with_name("staging"), target, -1
                )
            self.assertEqual(raised.exception.errno, errno.ENOTSUP)

    def test_unsupported_platform_maps_to_typed_publication_failure(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            destination = Path(temp_dir) / "corpus"
            with (
                mock.patch.object(snapshot_filesystem.sys, "platform", "unsupported"),
                self.assertRaises(SnapshotPublishError) as raised,
            ):
                publish_snapshot(destination, _write_complete)
            self.assertIs(
                raised.exception.failure,
                SnapshotPublishFailure.PUBLICATION_FAILED,
            )

    @unittest.skipUnless(sys.platform == "darwin", "requires macOS")
    def test_macos_no_replace_path_preserves_existing_destination(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            staging = root / "staging"
            target = root / "corpus"
            staging.mkdir()
            target.mkdir()
            (target / "sentinel").write_bytes(b"existing")
            descriptor = os.open(root, os.O_RDONLY | os.O_DIRECTORY)
            try:
                with self.assertRaises(FileExistsError):
                    snapshot_filesystem.atomic_rename_noreplace(
                        staging, target, descriptor
                    )
            finally:
                os.close(descriptor)
            self.assertEqual((target / "sentinel").read_bytes(), b"existing")

    def test_destination_race_before_atomic_rename_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            destination = root / "corpus"

            def race(staging: Path, target: Path, parent_descriptor: int) -> None:
                target.mkdir()
                (target / "sentinel").write_bytes(b"attacker")
                raise OSError("destination appeared")

            with (
                mock.patch(
                    "evaluate.multiformat_snapshot_publish._atomic_rename_noreplace",
                    side_effect=race,
                ),
                self.assertRaises(SnapshotPublishError),
            ):
                publish_snapshot(destination, _write_complete)
            self.assertEqual((destination / "sentinel").read_bytes(), b"attacker")

    def test_staging_race_before_atomic_rename_never_succeeds(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            destination = root / "corpus"
            replacement: Path | None = None

            def race(
                staging: Path,
                target: Path,
                parent_descriptor: int,
            ) -> None:
                nonlocal replacement
                shutil.rmtree(staging)
                staging.mkdir()
                (staging / "sentinel").write_bytes(b"attacker")
                replacement = staging
                raise OSError("staging replaced")

            with (
                mock.patch(
                    "evaluate.multiformat_snapshot_publish._atomic_rename_noreplace",
                    side_effect=race,
                ),
                self.assertRaises(SnapshotPublishError),
            ):
                publish_snapshot(destination, _write_complete)
            self.assertFalse(destination.exists())
            self.assertIsNotNone(replacement)
            assert replacement is not None
            self.assertEqual((replacement / "sentinel").read_bytes(), b"attacker")

    def test_staging_cleanup_race_preserves_replacement_sentinel(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            destination = root / "corpus"
            replacement: Path | None = None
            original_matches = snapshot_publish._matches

            def race(
                path: Path,
                identity: snapshot_publish._Identity,
                expected_mode: Callable[[int], bool],
            ) -> bool:
                nonlocal replacement
                result = original_matches(path, identity, expected_mode)
                if path.name.startswith(".corpus.stage-") and result:
                    shutil.rmtree(path)
                    path.mkdir()
                    (path / "sentinel").write_bytes(b"attacker")
                    replacement = path
                return result

            with (
                mock.patch(
                    "evaluate.multiformat_snapshot_publish._matches",
                    side_effect=race,
                ),
                self.assertRaisesRegex(PrimaryWriterError, "primary"),
            ):
                publish_snapshot(
                    destination,
                    lambda staging: self._raise_primary(staging),
                )
            self.assertIsNotNone(replacement)
            assert replacement is not None
            self.assertEqual((replacement / "sentinel").read_bytes(), b"attacker")

    def test_lock_cleanup_race_preserves_replacement_sentinel(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            destination = root / "corpus"
            lock = root / ".corpus.snapshot.lock"
            original_matches = snapshot_publish._matches

            def race(
                path: Path,
                identity: snapshot_publish._Identity,
                expected_mode: Callable[[int], bool],
            ) -> bool:
                result = original_matches(path, identity, expected_mode)
                if path.name == lock.name and result:
                    path.unlink()
                    path.write_bytes(b"attacker")
                return result

            with (
                mock.patch(
                    "evaluate.multiformat_snapshot_publish._matches",
                    side_effect=race,
                ),
                self.assertRaises(SnapshotPublishError),
            ):
                publish_snapshot(destination, _write_complete)
            self.assertEqual(lock.read_bytes(), b"attacker")

    def _raise_primary(self, staging: Path) -> None:
        (staging / "partial").write_bytes(b"partial")
        raise PrimaryWriterError("primary")


if __name__ == "__main__":
    unittest.main()
