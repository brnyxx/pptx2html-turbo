from __future__ import annotations

import os
import shutil
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from evaluate import multiformat_font_snapshot as font_snapshot
from evaluate.multiformat_font_snapshot import (
    FontSnapshotError,
    generate_font_snapshot,
    validate_font_snapshot,
)
from evaluate.multiformat_schema import JsonValue


class MultiFormatFontSnapshotStabilityTests(unittest.TestCase):
    def test_rejects_source_changed_after_copy(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source = root / "source"
            self._font(source / "font.ttf", b"font")
            output = root / "output"
            original_copy = shutil.copyfile

            def copy_and_change(
                source_path: str | os.PathLike[str],
                target: str | os.PathLike[str],
                **kwargs: bool,
            ) -> None:
                original_copy(source_path, target, **kwargs)
                Path(source_path).write_bytes(b"changed")

            with (
                mock.patch(
                    "evaluate.multiformat_font_snapshot.shutil.copyfile",
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
            original_copy = shutil.copyfile

            def copy_and_replace(
                source_path: str | os.PathLike[str],
                target: str | os.PathLike[str],
                **kwargs: bool,
            ) -> None:
                original_copy(source_path, target, **kwargs)
                Path(source_path).unlink()
                Path(source_path).write_bytes(b"font")

            with (
                mock.patch(
                    "evaluate.multiformat_font_snapshot.shutil.copyfile",
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
            original_copy = shutil.copyfile

            def copy_and_link(
                source_path: str | os.PathLike[str],
                target_path: str | os.PathLike[str],
                **kwargs: bool,
            ) -> None:
                original_copy(source_path, target_path, **kwargs)
                Path(source_path).unlink()
                Path(source_path).symlink_to(target)

            with (
                mock.patch(
                    "evaluate.multiformat_font_snapshot.shutil.copyfile",
                    side_effect=copy_and_link,
                ),
                self.assertRaises(FontSnapshotError),
            ):
                generate_font_snapshot((source,), output)
            self.assertFalse(output.exists())

    def test_rejects_same_byte_output_inode_replacement_during_validation(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source = root / "source"
            self._font(source / "font.ttf", b"font")
            output = root / "output"
            generate_font_snapshot((source,), output)
            font = next((output / "fonts").iterdir()).resolve()
            original_hash = font_snapshot.sha256_file

            def hash_and_replace(path: Path) -> str:
                digest = original_hash(path)
                if path == font:
                    bytes_value = path.read_bytes()
                    path.unlink()
                    path.write_bytes(bytes_value)
                return digest

            with (
                mock.patch(
                    "evaluate.multiformat_font_snapshot.sha256_file",
                    side_effect=hash_and_replace,
                ),
                self.assertRaises(FontSnapshotError),
            ):
                validate_font_snapshot(output / "font-bundle.json", output)

    def test_rejects_same_byte_manifest_inode_replacement(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source = root / "source"
            self._font(source / "font.ttf", b"font")
            output = root / "output"
            generate_font_snapshot((source,), output)
            manifest = output / "font-bundle.json"
            original_read = font_snapshot.read_strict_object

            def read_and_replace(path: Path) -> dict[str, JsonValue]:
                values = original_read(path)
                bytes_value = path.read_bytes()
                path.unlink()
                path.write_bytes(bytes_value)
                return values

            with (
                mock.patch(
                    "evaluate.multiformat_font_snapshot.read_strict_object",
                    side_effect=read_and_replace,
                ),
                self.assertRaises(FontSnapshotError),
            ):
                validate_font_snapshot(manifest, output)

    def _font(self, path: Path, data: bytes) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)


if __name__ == "__main__":
    unittest.main()
