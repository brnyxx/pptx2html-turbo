from __future__ import annotations

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


class MultiFormatFontSnapshotValidationRaceTests(unittest.TestCase):
    def test_rejects_same_byte_output_inode_replacement_during_validation(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source = root / "source"
            self._font(source / "font.ttf", b"font")
            output = root / "output"
            generate_font_snapshot((source,), output)
            font = next((output / "fonts").iterdir()).resolve()
            original_read = font_snapshot.font_filesystem.read_stable_file
            replaced = False

            def read_and_replace(
                path: Path,
            ) -> font_snapshot.font_filesystem.StableFile:
                nonlocal replaced
                result = original_read(path)
                if path == font and not replaced:
                    replaced = True
                    bytes_value = path.read_bytes()
                    path.unlink()
                    path.write_bytes(bytes_value)
                return result

            with (
                mock.patch(
                    "evaluate.multiformat_font_filesystem.read_stable_file",
                    side_effect=read_and_replace,
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
            original_parse = font_snapshot.parse_strict_object_bytes

            def parse_and_replace(data: bytes) -> dict[str, JsonValue]:
                values = original_parse(data)
                manifest.unlink()
                manifest.write_bytes(data)
                return values

            with (
                mock.patch(
                    "evaluate.multiformat_font_snapshot.parse_strict_object_bytes",
                    side_effect=parse_and_replace,
                ),
                self.assertRaises(FontSnapshotError),
            ):
                validate_font_snapshot(manifest, output)

    def test_rejects_manifest_mutation_after_candidate_validation(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source = root / "source"
            self._font(source / "font.ttf", b"font")
            output = root / "output"
            generate_font_snapshot((source,), output)
            manifest = output / "font-bundle.json"
            original_validate = font_snapshot.validate_font_bundle

            def validate_then_mutate(path: Path) -> str:
                result = original_validate(path)
                bytes_value = path.read_bytes()
                marker = bytes_value.index(b'"schema_version":1') + len(
                    b'"schema_version":'
                )
                with path.open("r+b") as stream:
                    stream.seek(marker)
                    stream.write(b"2")
                return result

            with (
                mock.patch(
                    "evaluate.multiformat_font_snapshot.validate_font_bundle",
                    side_effect=validate_then_mutate,
                ),
                self.assertRaises(FontSnapshotError),
            ):
                validate_font_snapshot(manifest, output)

    def test_rejects_font_mutation_after_candidate_validation(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source = root / "source"
            self._font(source / "font.ttf", b"font")
            output = root / "output"
            generate_font_snapshot((source,), output)
            font = next((output / "fonts").iterdir()).resolve()
            original_validate = font_snapshot.validate_font_bundle

            def validate_then_mutate(path: Path) -> str:
                result = original_validate(path)
                with font.open("r+b") as stream:
                    stream.seek(0)
                    stream.write(b"X")
                return result

            with (
                mock.patch(
                    "evaluate.multiformat_font_snapshot.validate_font_bundle",
                    side_effect=validate_then_mutate,
                ),
                self.assertRaises(FontSnapshotError),
            ):
                validate_font_snapshot(output / "font-bundle.json", output)

    def _font(self, path: Path, data: bytes) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)


if __name__ == "__main__":
    unittest.main()
