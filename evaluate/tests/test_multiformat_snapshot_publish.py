from __future__ import annotations

import shutil
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from evaluate.multiformat_snapshot_publish import (
    SnapshotPublishError,
    SnapshotPublishFailure,
    publish_snapshot,
)


class PrimaryWriterError(Exception):
    pass


class MultiFormatSnapshotPublishTests(unittest.TestCase):
    def test_complete_tree_is_renamed_without_ready_marker(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            destination = root / "corpus"

            def write_tree(staging: Path) -> None:
                (staging / "nested").mkdir()
                (staging / "nested" / "manifest.json").write_bytes(b"complete")

            publish_snapshot(destination, write_tree)

            self.assertEqual(
                (destination / "nested" / "manifest.json").read_bytes(),
                b"complete",
            )
            self.assertFalse((destination / "READY").exists())
            self.assertEqual(tuple(root.glob(".corpus.stage-*")), ())
            self.assertFalse((root / ".corpus.snapshot.lock").exists())

    def test_existing_destination_and_lock_are_preserved(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            destination = root / "corpus"
            destination.mkdir()
            (destination / "sentinel").write_bytes(b"destination")
            writer_called = False

            def write_tree(staging: Path) -> None:
                nonlocal writer_called
                writer_called = True
                (staging / "unexpected").write_bytes(b"unexpected")

            with self.assertRaises(SnapshotPublishError) as raised:
                publish_snapshot(destination, write_tree)

            self.assertIs(
                raised.exception.failure,
                SnapshotPublishFailure.DESTINATION_EXISTS,
            )
            self.assertFalse(writer_called)
            self.assertEqual(
                (destination / "sentinel").read_bytes(),
                b"destination",
            )

        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            destination = root / "corpus"
            lock = root / ".corpus.snapshot.lock"
            lock.write_bytes(b"other")
            writer_called = False

            def write_locked_tree(staging: Path) -> None:
                nonlocal writer_called
                writer_called = True

            with self.assertRaises(SnapshotPublishError) as raised:
                publish_snapshot(destination, write_locked_tree)

            self.assertIs(raised.exception.failure, SnapshotPublishFailure.LOCKED)
            self.assertFalse(writer_called)
            self.assertEqual(lock.read_bytes(), b"other")
            self.assertFalse(destination.exists())

    def test_substituted_staging_inode_is_never_published_or_deleted(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            destination = root / "corpus"
            replacement: Path | None = None

            def substitute_staging(staging: Path) -> None:
                nonlocal replacement
                shutil.rmtree(staging)
                staging.mkdir()
                (staging / "sentinel").write_bytes(b"other")
                replacement = staging

            with self.assertRaises(SnapshotPublishError) as raised:
                publish_snapshot(destination, substitute_staging)

            self.assertIs(
                raised.exception.failure,
                SnapshotPublishFailure.PUBLICATION_FAILED,
            )
            self.assertFalse(destination.exists())
            self.assertIsNotNone(replacement)
            assert replacement is not None
            self.assertEqual((replacement / "sentinel").read_bytes(), b"other")
            self.assertFalse((root / ".corpus.snapshot.lock").exists())

    def test_cleanup_error_adds_note_without_masking_primary_error(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            destination = root / "corpus"

            with (
                mock.patch(
                    "evaluate.multiformat_snapshot_publish._unlink_owned_file",
                    side_effect=OSError("cleanup"),
                ),
                self.assertRaisesRegex(PrimaryWriterError, "primary") as raised,
            ):
                publish_snapshot(
                    destination,
                    lambda staging: self._raise_primary(staging),
                )

            self.assertTrue(
                any("cleanup" in note for note in raised.exception.__notes__)
            )
            self.assertFalse(destination.exists())
            self.assertEqual(tuple(root.glob(".corpus.stage-*")), ())

    def test_standalone_cleanup_error_is_typed(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            destination = root / "corpus"

            with (
                mock.patch(
                    "evaluate.multiformat_snapshot_publish._unlink_owned_file",
                    side_effect=OSError("cleanup"),
                ),
                self.assertRaises(SnapshotPublishError) as raised,
            ):
                publish_snapshot(
                    destination,
                    lambda staging: (staging / "manifest.json").write_bytes(
                        b"complete"
                    ),
                )

            self.assertIs(
                raised.exception.failure,
                SnapshotPublishFailure.PUBLICATION_FAILED,
            )
            self.assertEqual(
                raised.exception.path,
                root.resolve() / ".corpus.snapshot.lock",
            )
            self.assertIsInstance(raised.exception.__cause__, OSError)
            self.assertEqual(
                (destination / "manifest.json").read_bytes(),
                b"complete",
            )

    def _raise_primary(self, staging: Path) -> None:
        (staging / "partial").write_bytes(b"partial")
        raise PrimaryWriterError("primary")


if __name__ == "__main__":
    unittest.main()
