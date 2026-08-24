from __future__ import annotations

import os
import tempfile
import unittest
from collections.abc import Callable
from pathlib import Path
from typing import cast
from unittest.mock import patch

from evaluate import multiformat_ready_tree_fs
from evaluate.multiformat_ready_tree import TreeIdentityError, tree_identity


class MultiFormatReadyTreeRaceTests(unittest.TestCase):
    def test_primary_tree_error_survives_close_failure_with_note(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            target = root / "target.txt"
            _ = target.write_bytes(b"target")
            _ = (root / "invalid-link").symlink_to(target)

            def fail_close(_descriptor: int) -> None:
                raise OSError("injected close failure")

            with (
                patch.object(os, "close", side_effect=fail_close),
                self.assertRaises(TreeIdentityError) as raised,
            ):
                _ = tree_identity(root)

            self.assertIn("symlink is not allowed", str(raised.exception))
            self.assertTrue(
                any(
                    "injected close failure" in note
                    for note in raised.exception.__notes__
                )
            )

    def test_standalone_close_failure_is_typed(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)

            def fail_close(_descriptor: int) -> None:
                raise OSError("injected close failure")

            with (
                patch.object(os, "close", side_effect=fail_close),
                self.assertRaises(TreeIdentityError) as raised,
            ):
                _ = tree_identity(root)

            self.assertIn("cannot close tree entry", str(raised.exception))
            self.assertIsInstance(raised.exception, TreeIdentityError)

    def test_post_second_read_equal_size_mutation_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source = root / "source.bin"
            _ = source.write_bytes(b"a" * (2 * 1024 * 1024))
            read_count = 0
            real_read = cast(
                Callable[[int, str], tuple[str, os.stat_result, os.stat_result]],
                getattr(multiformat_ready_tree_fs, "_read_descriptor_hash"),
            )

            def read_then_mutate(
                descriptor: int,
                relative_path: str,
            ) -> tuple[str, os.stat_result, os.stat_result]:
                nonlocal read_count
                result = real_read(descriptor, relative_path)
                read_count += 1
                if read_count == 2:
                    with source.open("r+b") as stream:
                        _ = stream.seek(1024 * 1024)
                        _ = stream.write(b"b" * (1024 * 1024))
                return result

            with (
                patch.object(
                    multiformat_ready_tree_fs,
                    "_read_descriptor_hash",
                    side_effect=read_then_mutate,
                ),
                self.assertRaises(TreeIdentityError),
            ):
                _ = tree_identity(root)

            self.assertEqual(read_count, 2)


if __name__ == "__main__":
    _ = unittest.main()
