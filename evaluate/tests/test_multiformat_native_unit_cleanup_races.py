from __future__ import annotations

import os
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_native_unit_cleanup import cleanup_workspace, identity
from evaluate.multiformat_native_unit_types import NativeUnitError
from evaluate.tests.multiformat_native_unit_fixture import make_native_unit_fixture


class MultiFormatNativeUnitCleanupRaceTests(unittest.TestCase):
    def test_final_unlink_race_preserves_captured_attacker_and_owned_backup(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_unit_fixture(root)
            request = fixture.request(root, DocumentFormat.PDF)
            workspace = root / "workspace"
            workspace.mkdir()
            owned = workspace / "owned"
            _ = owned.write_bytes(b"owned")
            expected = identity(workspace.lstat())
            backup = root / "owned-backup"
            held_tombstone = -1
            attacker_name = ".captured-entry"

            def race(directory_descriptor: int, name: str) -> None:
                nonlocal held_tombstone
                if name == attacker_name and held_tombstone < 0:
                    held_tombstone = os.dup(directory_descriptor)
                    os.rename(name, backup, src_dir_fd=directory_descriptor)
                    descriptor = os.open(
                        name,
                        os.O_WRONLY | os.O_CREAT | os.O_EXCL,
                        0o600,
                        dir_fd=directory_descriptor,
                    )
                    os.close(descriptor)

            try:
                with (
                    patch(
                        "evaluate.multiformat_native_unit_cleanup._before_tombstone_remove",
                        side_effect=race,
                    ),
                    self.assertRaises(NativeUnitError),
                ):
                    cleanup_workspace(workspace, expected, request)
                self.assertEqual(backup.read_bytes(), b"owned")
                self.assertTrue(_entry_exists(held_tombstone, attacker_name))
            finally:
                if held_tombstone >= 0:
                    os.close(held_tombstone)

    def test_final_root_rmdir_race_preserves_captured_attacker_and_owned_backup(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_unit_fixture(root)
            request = fixture.request(root, DocumentFormat.PDF)
            workspace = root / "workspace"
            workspace.mkdir()
            _ = (workspace / "owned").write_bytes(b"owned")
            expected = identity(workspace.lstat())
            backup = root / "workspace-backup"
            held_tombstone = -1
            attacker_name = ".captured-entry"

            def race(directory_descriptor: int, name: str) -> None:
                nonlocal held_tombstone
                if name == attacker_name and held_tombstone < 0:
                    try:
                        probe = os.open(
                            name,
                            os.O_RDONLY | getattr(os, "O_DIRECTORY", 0),
                            dir_fd=directory_descriptor,
                        )
                    except OSError:
                        return
                    os.close(probe)
                    held_tombstone = os.dup(directory_descriptor)
                    os.rename(name, backup, src_dir_fd=directory_descriptor)
                    os.mkdir(name, 0o700, dir_fd=directory_descriptor)

            try:
                with (
                    patch(
                        "evaluate.multiformat_native_unit_cleanup._before_tombstone_remove",
                        side_effect=race,
                    ),
                    self.assertRaises(NativeUnitError),
                ):
                    cleanup_workspace(workspace, expected, request)
                self.assertTrue(backup.is_dir())
                self.assertEqual(tuple(backup.iterdir()), ())
                self.assertTrue(_entry_exists(held_tombstone, attacker_name))
            finally:
                if held_tombstone >= 0:
                    os.close(held_tombstone)

    def test_inner_capture_race_restores_attacker_and_preserves_owned_data(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_unit_fixture(root)
            request = fixture.request(root, DocumentFormat.PDF)
            workspace = root / "workspace"
            workspace.mkdir()
            tombstone = workspace / "tombstone"
            _ = tombstone.write_bytes(b"owned")
            expected = identity(workspace.lstat())
            backup = root / "tombstone-backup"
            attacker = workspace / "tombstone"
            swapped = False

            def race(directory_descriptor: int, name: str) -> None:
                nonlocal swapped
                if not swapped and name == "tombstone":
                    swapped = True
                    os.rename(name, backup, src_dir_fd=directory_descriptor)
                    descriptor = os.open(
                        name,
                        os.O_WRONLY | os.O_CREAT | os.O_EXCL,
                        0o600,
                        dir_fd=directory_descriptor,
                    )
                    try:
                        _ = os.write(descriptor, b"attacker")
                    finally:
                        os.close(descriptor)

            with (
                patch(
                    "evaluate.multiformat_native_unit_cleanup._before_inner_delete",
                    side_effect=race,
                ),
                self.assertRaises(NativeUnitError),
            ):
                cleanup_workspace(workspace, expected, request)

            self.assertTrue(swapped)
            self.assertEqual(attacker.read_bytes(), b"attacker")
            self.assertEqual(backup.read_bytes(), b"owned")
            self.assertTrue(workspace.is_dir())


def _entry_exists(directory_descriptor: int, name: str) -> bool:
    try:
        _ = os.lstat(name, dir_fd=directory_descriptor)
    except (FileNotFoundError, OSError):
        return False
    return True


if __name__ == "__main__":
    _ = unittest.main()
