from __future__ import annotations

import shutil
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from evaluate.multiformat_security_publish import (
    SecurityPublishError,
    SecurityPublishFailure,
    publish_security_snapshot,
)


class PrimaryWriterError(Exception):
    pass


class MultiFormatSecurityPublishTests(unittest.TestCase):
    def test_cleanup_failure_never_masks_primary_writer_failure(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            destination = root / "security"

            with (
                mock.patch(
                    "evaluate.multiformat_security_publish._unlink_owned_file",
                    side_effect=OSError("cleanup"),
                ),
                self.assertRaisesRegex(PrimaryWriterError, "primary") as raised,
            ):
                publish_security_snapshot(
                    destination,
                    lambda staging: self._raise_primary(staging),
                )

            self.assertTrue(
                any("cleanup" in note for note in raised.exception.__notes__)
            )
            self.assertFalse(destination.exists())

    def test_writer_failure_releases_owned_staging_and_lock(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            destination = root / "security"

            with self.assertRaisesRegex(OSError, "injected"):
                publish_security_snapshot(
                    destination,
                    lambda staging: self._fail(staging),
                )

            self.assertFalse(destination.exists())
            self.assertEqual(tuple(root.glob(".security.stage-*")), ())
            self.assertFalse(self._lock(root).exists())

    def test_substituted_staging_directory_is_not_deleted(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            destination = root / "security"
            replacement: Path | None = None

            def substitute(staging: Path) -> None:
                nonlocal replacement
                shutil.rmtree(staging)
                staging.mkdir()
                (staging / "sentinel").write_bytes(b"other")
                replacement = staging
                raise OSError("injected")

            with self.assertRaisesRegex(OSError, "injected"):
                publish_security_snapshot(destination, substitute)

            self.assertIsNotNone(replacement)
            assert replacement is not None
            self.assertEqual((replacement / "sentinel").read_bytes(), b"other")
            self.assertFalse(self._lock(root).exists())

    def test_substituted_staging_directory_is_never_published(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            destination = root / "security"
            replacement: Path | None = None

            def substitute(staging: Path) -> None:
                nonlocal replacement
                shutil.rmtree(staging)
                staging.mkdir()
                (staging / "sentinel").write_bytes(b"other")
                replacement = staging

            with self.assertRaises(SecurityPublishError) as raised:
                publish_security_snapshot(destination, substitute)

            self.assertIs(
                raised.exception.failure,
                SecurityPublishFailure.PUBLICATION_FAILED,
            )
            self.assertFalse(destination.exists())
            self.assertIsNotNone(replacement)
            assert replacement is not None
            self.assertEqual((replacement / "sentinel").read_bytes(), b"other")
            self.assertFalse(self._lock(root).exists())

    def test_destination_appearing_before_rename_is_preserved(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            destination = root / "security"

            def occupy(staging: Path) -> None:
                (staging / "source").write_bytes(b"staged")
                destination.mkdir()
                (destination / "sentinel").write_bytes(b"other")

            with self.assertRaises(SecurityPublishError) as raised:
                publish_security_snapshot(destination, occupy)

            self.assertIs(
                raised.exception.failure,
                SecurityPublishFailure.DESTINATION_EXISTS,
            )
            self.assertEqual(
                (destination / "sentinel").read_bytes(),
                b"other",
            )
            self.assertEqual(tuple(root.glob(".security.stage-*")), ())
            self.assertFalse(self._lock(root).exists())

    def test_substituted_lock_is_not_unlinked(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            destination = root / "security"
            lock = self._lock(root)

            def substitute_lock(staging: Path) -> None:
                lock.unlink()
                lock.write_bytes(b"other")
                raise OSError("injected")

            with self.assertRaisesRegex(OSError, "injected"):
                publish_security_snapshot(destination, substitute_lock)

            self.assertEqual(lock.read_bytes(), b"other")
            self.assertEqual(tuple(root.glob(".security.stage-*")), ())

    def _fail(self, staging: Path) -> None:
        (staging / "partial").write_bytes(b"partial")
        raise OSError("injected")

    def _raise_primary(self, staging: Path) -> None:
        (staging / "partial").write_bytes(b"partial")
        raise PrimaryWriterError("primary")

    def _lock(self, root: Path) -> Path:
        return root / ".security.security-sources.lock"


if __name__ == "__main__":
    unittest.main()
