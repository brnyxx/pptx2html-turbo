from __future__ import annotations

import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_native_unit_cleanup import cleanup_workspace, identity
from evaluate.multiformat_native_unit_types import NativeUnitError
from evaluate.tests.multiformat_native_unit_fixture import make_native_unit_fixture


class MultiFormatNativeUnitCleanupRaceTests(unittest.TestCase):
    def test_inner_delete_race_preserves_tombstone_and_attacker(self) -> None:
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

            def race(_directory_descriptor: int, name: str) -> None:
                nonlocal swapped
                if not swapped and name == "tombstone":
                    swapped = True
                    _ = tombstone.rename(backup)
                    _ = tombstone.write_bytes(b"attacker")

            with (
                patch(
                    "evaluate.multiformat_native_unit_cleanup._before_inner_delete",
                    side_effect=race,
                ),
                self.assertRaises(NativeUnitError) as raised,
            ):
                cleanup_workspace(workspace, expected, request)

            self.assertTrue(swapped)
            self.assertEqual(attacker.read_bytes(), b"attacker")
            self.assertEqual(backup.read_bytes(), b"owned")
            self.assertTrue(workspace.is_dir())
            self.assertEqual(raised.exception.failure.value, "output-invalid")


if __name__ == "__main__":
    _ = unittest.main()
