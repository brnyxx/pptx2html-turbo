from __future__ import annotations

import os
import socket
import stat
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from evaluate import multiformat_public_pool_fs
from evaluate.multiformat_public_pool import load_validated_public_pool_sources
from evaluate.multiformat_public_pool_types import PublicPoolError
from evaluate.tests.multiformat_public_pool_fixture import (
    write_multiformat_public_pool_fixture,
)


class MultiFormatPublicPoolEntryTests(unittest.TestCase):
    @unittest.skipUnless(os.name == "posix", "requires POSIX filesystem entries")
    def test_extra_fifo_is_rejected_and_untouched(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            fixture = write_multiformat_public_pool_fixture(Path(temp_dir))
            attacker = fixture.manifest.parent / "attacker-fifo"
            os.mkfifo(attacker)
            before = _entry_identity(attacker)
            classifications: list[int] = []

            with (
                patch.object(
                    multiformat_public_pool_fs,
                    "_before_special_entry_rejection",
                    side_effect=classifications.append,
                    create=True,
                ),
                self.assertRaises(PublicPoolError),
            ):
                _ = load_validated_public_pool_sources(
                    fixture.config,
                    fixture.manifest,
                )

            self.assertEqual(len(classifications), 1)
            self.assertTrue(stat.S_ISFIFO(classifications[0]))
            self.assertTrue(stat.S_ISFIFO(attacker.lstat().st_mode))
            self.assertEqual(_entry_identity(attacker), before)

    @unittest.skipUnless(os.name == "posix", "requires POSIX filesystem entries")
    def test_extra_unix_socket_is_rejected_and_untouched(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            fixture = write_multiformat_public_pool_fixture(Path(temp_dir))
            attacker = fixture.manifest.parent / "attacker.sock"
            listener = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            listener.bind(str(attacker))
            before = _entry_identity(attacker)
            classifications: list[int] = []
            try:
                with (
                    patch.object(
                        multiformat_public_pool_fs,
                        "_before_special_entry_rejection",
                        side_effect=classifications.append,
                        create=True,
                    ),
                    self.assertRaises(PublicPoolError),
                ):
                    _ = load_validated_public_pool_sources(
                        fixture.config,
                        fixture.manifest,
                    )

                self.assertEqual(len(classifications), 1)
                self.assertTrue(stat.S_ISSOCK(classifications[0]))
                self.assertTrue(stat.S_ISSOCK(attacker.lstat().st_mode))
                self.assertEqual(_entry_identity(attacker), before)
            finally:
                listener.close()

    def test_extra_empty_directory_is_rejected_and_untouched(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            fixture = write_multiformat_public_pool_fixture(Path(temp_dir))
            attacker = fixture.manifest.parent / "attacker-directory"
            attacker.mkdir()
            before = _entry_identity(attacker)

            with self.assertRaises(PublicPoolError):
                _ = load_validated_public_pool_sources(
                    fixture.config,
                    fixture.manifest,
                )

            self.assertTrue(attacker.is_dir())
            self.assertEqual(_entry_identity(attacker), before)


def _entry_identity(path: Path) -> tuple[int, int, int]:
    value = path.lstat()
    return value.st_dev, value.st_ino, value.st_mode


if __name__ == "__main__":
    _ = unittest.main()
