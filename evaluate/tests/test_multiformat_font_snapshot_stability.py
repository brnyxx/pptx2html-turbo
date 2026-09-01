from __future__ import annotations

import os
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from evaluate import multiformat_font_filesystem as font_filesystem
from evaluate.multiformat_font_snapshot import (
    FontSnapshotError,
    generate_font_snapshot,
)


class MultiFormatFontSnapshotStabilityTests(unittest.TestCase):
    def test_rejects_root_replacement_during_discovery(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source = root / "source"
            self._font(source / "font.ttf", b"font")
            source = source.resolve()
            output = root / "output"
            original_open = os.open
            replaced = False

            def replace_root(
                path: str | os.PathLike[str],
                flags: int,
                mode: int = 0o777,
                *,
                dir_fd: int | None = None,
            ) -> int:
                nonlocal replaced
                if Path(path) == source and dir_fd is None and not replaced:
                    replaced = True
                    source.rename(root / "original-source")
                    self._font(source / "attacker.ttf", b"attacker")
                return original_open(path, flags, mode, dir_fd=dir_fd)

            with (
                mock.patch(
                    "evaluate.multiformat_font_filesystem.os.open",
                    side_effect=replace_root,
                ),
                self.assertRaises(FontSnapshotError),
            ):
                generate_font_snapshot((source,), output)
            self.assertFalse(output.exists())

    def test_rejects_checked_directory_replacement_during_discovery(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source = root / "source"
            checked = source / "checked"
            self._font(checked / "font.ttf", b"font")
            source = source.resolve()
            checked = checked.resolve()
            output = root / "output"
            original_open = os.open
            replaced = False

            def replace_checked(
                path: str | os.PathLike[str],
                flags: int,
                mode: int = 0o777,
                *,
                dir_fd: int | None = None,
            ) -> int:
                nonlocal replaced
                if Path(path) == checked and dir_fd is None and not replaced:
                    replaced = True
                    checked.rename(root / "original-checked")
                    self._font(checked / "attacker.ttf", b"attacker")
                return original_open(path, flags, mode, dir_fd=dir_fd)

            with (
                mock.patch(
                    "evaluate.multiformat_font_filesystem.os.open",
                    side_effect=replace_checked,
                ),
                self.assertRaises(FontSnapshotError),
            ):
                generate_font_snapshot((source,), output)
            self.assertFalse(output.exists())

    def test_rejects_destination_symlink_without_overwriting_target(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source = root / "source"
            self._font(source / "font.ttf", b"font")
            output = root / "output"
            outside = root / "outside.ttf"
            outside.write_bytes(b"outside")
            original_copy = font_filesystem.copy_font_file

            def link_destination(
                source_path: Path,
                expected_identity: tuple[int, int],
                expected_digest: str,
                target_path: Path,
            ) -> None:
                target_path.symlink_to(outside)
                original_copy(
                    source_path,
                    expected_identity,
                    expected_digest,
                    target_path,
                )

            with (
                mock.patch(
                    "evaluate.multiformat_font_filesystem.copy_font_file",
                    side_effect=link_destination,
                ),
                self.assertRaises(FontSnapshotError),
            ):
                generate_font_snapshot((source,), output)
            self.assertEqual(outside.read_bytes(), b"outside")

    def test_rejects_source_changed_after_copy(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source = root / "source"
            self._font(source / "font.ttf", b"font")
            output = root / "output"
            original_copy = font_filesystem.copy_font_file

            def copy_and_change(
                source_path: Path,
                expected_identity: tuple[int, int],
                expected_digest: str,
                target: Path,
            ) -> None:
                original_copy(source_path, expected_identity, expected_digest, target)
                source_path.write_bytes(b"changed")

            with (
                mock.patch(
                    "evaluate.multiformat_font_filesystem.copy_font_file",
                    side_effect=copy_and_change,
                ),
                self.assertRaises(FontSnapshotError),
            ):
                generate_font_snapshot((source,), output)
            self.assertFalse(output.exists())

    def test_rejects_same_byte_source_inode_replacement(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source = root / "source"
            source_file = source / "font.ttf"
            self._font(source_file, b"font")
            output = root / "output"
            original_copy = font_filesystem.copy_font_file

            def copy_and_replace(
                source_path: Path,
                expected_identity: tuple[int, int],
                expected_digest: str,
                target: Path,
            ) -> None:
                original_copy(source_path, expected_identity, expected_digest, target)
                source_path.unlink()
                source_path.write_bytes(b"font")

            with (
                mock.patch(
                    "evaluate.multiformat_font_filesystem.copy_font_file",
                    side_effect=copy_and_replace,
                ),
                self.assertRaises(FontSnapshotError),
            ):
                generate_font_snapshot((source,), output)
            self.assertFalse(output.exists())

    def test_rejects_source_symlinked_after_copy(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source = root / "source"
            source_file = source / "font.ttf"
            self._font(source_file, b"font")
            target = root / "target.ttf"
            target.write_bytes(b"font")
            output = root / "output"
            original_copy = font_filesystem.copy_font_file

            def copy_and_link(
                source_path: Path,
                expected_identity: tuple[int, int],
                expected_digest: str,
                target_path: Path,
            ) -> None:
                original_copy(
                    source_path,
                    expected_identity,
                    expected_digest,
                    target_path,
                )
                source_path.unlink()
                source_path.symlink_to(target)

            with (
                mock.patch(
                    "evaluate.multiformat_font_filesystem.copy_font_file",
                    side_effect=copy_and_link,
                ),
                self.assertRaises(FontSnapshotError),
            ):
                generate_font_snapshot((source,), output)
            self.assertFalse(output.exists())

    def _font(self, path: Path, data: bytes) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)


if __name__ == "__main__":
    unittest.main()
