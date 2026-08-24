from __future__ import annotations

import hashlib
import json
import os
import tempfile
import unittest
from collections.abc import Callable
from pathlib import Path
from unittest import mock

from evaluate.multiformat_font_snapshot import (
    FontSnapshotError,
    generate_font_snapshot,
    validate_font_snapshot,
)


class MultiFormatFontSnapshotTests(unittest.TestCase):
    def test_materializes_exact_canonical_bundle_and_validates_it(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            first = root / "first"
            second = root / "second"
            self._font(first / "z.ttf", b"font-z")
            self._font(second / "a.otf", b"font-a")
            output = root / "bundle"

            summary = generate_font_snapshot((first, second), output)
            digest_a = hashlib.sha256(b"font-a").hexdigest()
            digest_z = hashlib.sha256(b"font-z").hexdigest()
            ordered = sorted(
                ((digest_a, ".otf"), (digest_z, ".ttf")),
                key=lambda item: (item[0], item[1]),
            )
            manifest_value = {
                "schema_version": 1,
                "fonts": [
                    {
                        "path": f"fonts/{ordinal:04d}-{digest}{suffix}",
                        "sha256": digest,
                    }
                    for ordinal, (digest, suffix) in enumerate(ordered, start=1)
                ],
            }
            expected = (
                json.dumps(
                    manifest_value,
                    ensure_ascii=False,
                    separators=(",", ":"),
                    sort_keys=True,
                ).encode()
                + b"\n"
            )

            self.assertEqual((output / "font-bundle.json").read_bytes(), expected)
            self.assertEqual(
                sorted(
                    path.relative_to(output).as_posix() for path in output.rglob("*")
                ),
                [
                    "font-bundle.json",
                    "fonts",
                    *[
                        f"fonts/{ordinal:04d}-{digest}{suffix}"
                        for ordinal, (digest, suffix) in enumerate(ordered, start=1)
                    ],
                ],
            )
            self.assertEqual(
                summary, validate_font_snapshot(output / "font-bundle.json", output)
            )
            self.assertEqual(summary.files, 3)
            self.assertEqual(summary.fonts, 2)
            self.assertFalse((output / "READY").exists())

    def test_source_directory_order_does_not_change_tree_bytes(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            first = root / "first"
            second = root / "second"
            self._font(first / "one.ttf", b"one")
            self._font(second / "two.otf", b"two")
            left = root / "left"
            right = root / "right"

            generate_font_snapshot((first, second), left)
            generate_font_snapshot((second, first), right)

            self.assertEqual(
                sorted(path.relative_to(left).as_posix() for path in left.rglob("*")),
                sorted(path.relative_to(right).as_posix() for path in right.rglob("*")),
            )
            for left_path in left.rglob("*"):
                if left_path.is_file():
                    self.assertEqual(
                        left_path.read_bytes(),
                        (right / left_path.relative_to(left)).read_bytes(),
                    )

    def test_rejects_links_hard_links_duplicates_special_files_and_suffixes(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            cases: list[tuple[str, Callable[[], None]]] = [
                ("symlink-root", lambda: self._symlink_root(root)),
                ("symlink-entry", lambda: self._symlink_entry(root)),
                ("hard-link", lambda: self._hard_link(root)),
                ("repeated-inode", lambda: self._repeated_inode(root)),
                ("duplicate-digest", lambda: self._duplicate_digest(root)),
                ("unsupported", lambda: self._unsupported(root)),
                ("special", lambda: self._special(root)),
            ]
            for name, setup in cases:
                with self.subTest(name=name):
                    setup()
                    with self.assertRaises(FontSnapshotError):
                        generate_font_snapshot((root / name,), root / f"{name}-out")

    def test_rejects_noncanonical_manifest_bytes(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source = root / "source"
            self._font(source / "font.ttf", b"font")
            output = root / "output"
            generate_font_snapshot((source,), output)
            manifest = output / "font-bundle.json"
            values = json.loads(manifest.read_text())
            manifest.write_text(json.dumps(values, indent=2) + "\n")

            with self.assertRaises(FontSnapshotError):
                validate_font_snapshot(manifest, output)

    def test_rejects_unsafe_manifest_paths_before_resolution(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source = root / "source"
            self._font(source / "font.ttf", b"font")
            output = root / "output"
            generate_font_snapshot((source,), output)
            manifest = output / "font-bundle.json"
            values = json.loads(manifest.read_text())
            values["fonts"][0]["path"] = "fonts/../outside.ttf"
            manifest.write_text(json.dumps(values))

            with self.assertRaises(FontSnapshotError):
                validate_font_snapshot(manifest, output)

    def test_rejects_extra_changed_and_linked_output_entries(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source = root / "source"
            self._font(source / "font.ttf", b"font")
            output = root / "output"
            generate_font_snapshot((source,), output)
            (output / "extra").write_bytes(b"extra")
            with self.assertRaises(FontSnapshotError):
                validate_font_snapshot(output / "font-bundle.json", output)

            (output / "extra").unlink()
            font = next((output / "fonts").iterdir())
            font.write_bytes(b"changed")
            with self.assertRaises(FontSnapshotError):
                validate_font_snapshot(output / "font-bundle.json", output)

    def test_writer_failure_leaves_no_destination(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source = root / "source"
            self._font(source / "font.ttf", b"font")
            output = root / "output"
            with (
                mock.patch(
                    "evaluate.multiformat_font_snapshot.shutil.copyfile",
                    side_effect=OSError("copy failed"),
                ),
                self.assertRaises(FontSnapshotError),
            ):
                generate_font_snapshot((source,), output)
            self.assertFalse(output.exists())

    def test_existing_destination_or_lock_publish_nothing(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source = root / "source"
            self._font(source / "font.ttf", b"font")
            existing = root / "existing"
            existing.mkdir()
            (existing / "sentinel").write_bytes(b"keep")
            with self.assertRaises(FontSnapshotError):
                generate_font_snapshot((source,), existing)
            self.assertEqual((existing / "sentinel").read_bytes(), b"keep")

            locked = root / "locked"
            (root / ".locked.snapshot.lock").write_bytes(b"lock")
            with self.assertRaises(FontSnapshotError):
                generate_font_snapshot((source,), locked)
            self.assertFalse(locked.exists())

    def _font(self, path: Path, data: bytes) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)

    def _symlink_root(self, root: Path) -> None:
        target = root / "symlink-root-target"
        self._font(target / "font.ttf", b"font")
        (root / "symlink-root").symlink_to(target, target_is_directory=True)

    def _symlink_entry(self, root: Path) -> None:
        target = root / "symlink-entry-target.ttf"
        self._font(target, b"font")
        directory = root / "symlink-entry"
        directory.mkdir()
        (directory / "font.ttf").symlink_to(target)

    def _hard_link(self, root: Path) -> None:
        directory = root / "hard-link"
        self._font(directory / "font.ttf", b"font")
        os.link(directory / "font.ttf", directory / "other.ttf")

    def _repeated_inode(self, root: Path) -> None:
        directory = root / "repeated-inode"
        self._font(directory / "font.ttf", b"font")
        (directory / "nested").mkdir()
        os.rename(directory / "font.ttf", directory / "nested" / "font.ttf")
        os.link(directory / "nested" / "font.ttf", directory / "font-copy.ttf")

    def _duplicate_digest(self, root: Path) -> None:
        directory = root / "duplicate-digest"
        self._font(directory / "one.ttf", b"same")
        self._font(directory / "two.otf", b"same")

    def _unsupported(self, root: Path) -> None:
        self._font(root / "unsupported" / "font.txt", b"font")

    def _special(self, root: Path) -> None:
        directory = root / "special"
        directory.mkdir()
        os.mkfifo(directory / "font.ttf")


if __name__ == "__main__":
    unittest.main()
