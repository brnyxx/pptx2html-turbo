from __future__ import annotations

import hashlib
import os
import tempfile
import unittest
from collections.abc import Iterator
from pathlib import Path
from typing import Protocol
from unittest.mock import patch

from evaluate.jcs import canonicalize
from evaluate.multiformat_ready_tree import TreeIdentityError, tree_identity
from evaluate.multiformat_schema import JsonValue


class _HashProtocol(Protocol):
    def update(self, data: bytes | bytearray | memoryview[int], /) -> None: ...

    def hexdigest(self) -> str: ...


class _MutatingHash:
    _inner: _HashProtocol
    _target: Path
    _replacement: bytes
    mutated: bool

    def __init__(self, inner: _HashProtocol, target: Path, replacement: bytes) -> None:
        self._inner = inner
        self._target = target
        self._replacement = replacement
        self.mutated = False

    def update(self, data: bytes | bytearray | memoryview[int], /) -> None:
        self._inner.update(data)
        if not self.mutated:
            self.mutated = True
            with self._target.open("r+b") as source:
                _ = source.seek(1024 * 1024)
                _ = source.write(self._replacement)

    def hexdigest(self) -> str:
        return self._inner.hexdigest()


class MultiFormatReadyTreeIdentityTests(unittest.TestCase):
    def test_empty_tree_has_canonical_empty_identity(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)

            result = tree_identity(root)

            expected = canonicalize({"schema_version": 1, "files": []})
            self.assertEqual(result.sha256, hashlib.sha256(expected).hexdigest())
            self.assertEqual(result.entry_count, 0)
            self.assertEqual(result.total_bytes, 0)

    def test_nested_files_are_sorted_by_utf8_path_bytes(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            files = {
                "é.txt": b"accent",
                "z.txt": b"z",
                "nested/b.txt": b"b",
                "nested/a.txt": b"a",
            }
            for relative_path, value in reversed(tuple(files.items())):
                path = root / relative_path
                _ = path.parent.mkdir(parents=True, exist_ok=True)
                _ = path.write_bytes(value)

            result = tree_identity(root)

            records: list[JsonValue] = [
                {
                    "path": relative_path,
                    "sha256": hashlib.sha256(value).hexdigest(),
                    "size": len(value),
                }
                for relative_path, value in sorted(
                    files.items(), key=lambda item: item[0].encode("utf-8")
                )
            ]
            expected_value: JsonValue = {"schema_version": 1, "files": records}
            expected = canonicalize(expected_value)
            self.assertEqual(result.sha256, hashlib.sha256(expected).hexdigest())
            self.assertEqual(result.entry_count, len(files))
            self.assertEqual(result.total_bytes, sum(map(len, files.values())))

    def test_byte_identical_trees_have_the_same_identity(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            first = Path(temp_dir) / "first"
            second = Path(temp_dir) / "second"
            files = {"b/data.bin": b"same", "a.txt": b"bytes"}
            for root, creation_order in (
                (first, tuple(files)),
                (second, tuple(reversed(tuple(files)))),
            ):
                for relative_path in creation_order:
                    path = root / relative_path
                    _ = path.parent.mkdir(parents=True, exist_ok=True)
                    _ = path.write_bytes(files[relative_path])

            first_identity = tree_identity(first)
            second_identity = tree_identity(second)
            self.assertEqual(first_identity, second_identity)

    def test_root_manifest_is_excluded_but_nested_manifest_is_bound(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            _ = (root / "data.txt").write_bytes(b"data")
            _ = (root / "assembly-manifest.json").write_bytes(b"first")

            first = tree_identity(root)
            _ = (root / "assembly-manifest.json").write_bytes(b"second")
            second = tree_identity(root)
            self.assertEqual(second, first)

            nested = root / "nested" / "assembly-manifest.json"
            _ = nested.parent.mkdir()
            _ = nested.write_bytes(b"nested")
            self.assertNotEqual(tree_identity(root), first)

    def test_path_and_file_mutations_change_identity(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            original = root / "original.txt"
            _ = original.write_bytes(b"original")
            before = tree_identity(root)

            _ = original.rename(root / "renamed.txt")
            path_mutated = tree_identity(root)
            self.assertNotEqual(path_mutated.sha256, before.sha256)

            _ = (root / "renamed.txt").write_bytes(b"changed")
            file_mutated = tree_identity(root)
            self.assertNotEqual(file_mutated.sha256, path_mutated.sha256)
            self.assertNotEqual(file_mutated.total_bytes, path_mutated.total_bytes)

    def test_invalid_file_types_are_rejected_deterministically(self) -> None:
        cases = ("symlink", "hard_link", "fifo")
        for case in cases:
            with self.subTest(case=case), tempfile.TemporaryDirectory() as temp_dir:
                root = Path(temp_dir)
                source = root / "source.txt"
                _ = source.write_bytes(b"source")
                invalid = root / "invalid"
                if case == "symlink":
                    _ = invalid.symlink_to(source)
                elif case == "hard_link":
                    _ = invalid.hardlink_to(source)
                else:
                    _ = os.mkfifo(invalid)

                with self.assertRaises(TreeIdentityError) as first_error:
                    _ = tree_identity(root)
                with self.assertRaises(TreeIdentityError) as second_error:
                    _ = tree_identity(root)

                self.assertEqual(
                    str(first_error.exception), str(second_error.exception)
                )
                self.assertIn("invalid", str(first_error.exception))

    def test_root_replacement_before_traversal_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            workspace = Path(temp_dir)
            root = workspace / "root"
            external = workspace / "external"
            root.mkdir()
            external.mkdir()
            _ = (root / "inside.txt").write_bytes(b"inside")
            _ = (external / "secret.txt").write_bytes(b"external")
            checked_root = workspace / "checked-root"
            replaced = False
            real_scandir = os.scandir

            def replace_root_before_scan(
                candidate: str | bytes | os.PathLike[str] | int,
            ) -> Iterator[os.DirEntry[str] | os.DirEntry[bytes]]:
                nonlocal replaced
                if not replaced:
                    replaced = True
                    _ = root.rename(checked_root)
                    _ = external.rename(root)
                return real_scandir(candidate)

            with (
                patch.object(os, "scandir", side_effect=replace_root_before_scan),
                self.assertRaises(TreeIdentityError),
            ):
                _ = tree_identity(root)

            self.assertTrue(replaced)
            self.assertTrue((root / "secret.txt").exists())

    def test_checked_directory_replacement_before_traversal_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            workspace = Path(temp_dir)
            root = workspace / "root"
            checked = root / "checked"
            external = workspace / "external"
            root.mkdir()
            checked.mkdir()
            external.mkdir()
            _ = (checked / "inside.txt").write_bytes(b"inside")
            _ = (external / "secret.txt").write_bytes(b"external")
            checked_backup = workspace / "checked-backup"
            scan_count = 0
            real_scandir = os.scandir

            def replace_checked_before_scan(
                candidate: str | bytes | os.PathLike[str] | int,
            ) -> Iterator[os.DirEntry[str] | os.DirEntry[bytes]]:
                nonlocal scan_count
                scan_count += 1
                if scan_count == 2:
                    _ = checked.rename(checked_backup)
                    _ = external.rename(checked)
                return real_scandir(candidate)

            with (
                patch.object(os, "scandir", side_effect=replace_checked_before_scan),
                self.assertRaises(TreeIdentityError),
            ):
                _ = tree_identity(root)

            self.assertEqual(scan_count, 2)
            self.assertTrue((checked / "secret.txt").exists())

    def test_equal_size_in_place_mutation_during_hashing_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source = root / "source.bin"
            original = b"a" * (2 * 1024 * 1024)
            replacement = b"b" * (1024 * 1024)
            _ = source.write_bytes(original)
            mutating_hashes: list[_MutatingHash] = []
            real_sha256 = hashlib.sha256

            def sha256_factory(data: bytes = b"") -> _HashProtocol:
                if not mutating_hashes:
                    mutating_hashes.append(
                        _MutatingHash(real_sha256(data), source, replacement)
                    )
                    return mutating_hashes[0]
                return real_sha256(data)

            with (
                patch.object(hashlib, "sha256", side_effect=sha256_factory),
                self.assertRaises(TreeIdentityError),
            ):
                _ = tree_identity(root)

            self.assertEqual(len(mutating_hashes), 1)
            self.assertTrue(mutating_hashes[0].mutated)

    def test_symlinked_directory_is_rejected_without_following_it(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            outside = Path(temp_dir) / "outside"
            _ = outside.mkdir()
            _ = (outside / "secret.txt").write_bytes(b"secret")
            _ = (root / "linked-dir").symlink_to(outside, target_is_directory=True)

            with self.assertRaises(TreeIdentityError):
                _ = tree_identity(root)

            self.assertTrue((outside / "secret.txt").exists())

    def test_non_directory_root_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir) / "file"
            _ = root.write_bytes(b"not a directory")

            with self.assertRaises(TreeIdentityError):
                _ = tree_identity(root)


if __name__ == "__main__":
    _ = unittest.main()
