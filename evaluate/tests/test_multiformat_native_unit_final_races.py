from __future__ import annotations

import os
import shutil
import tempfile
import unittest
from dataclasses import replace
from pathlib import Path
from unittest.mock import patch

from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_native_unit_files import (
    cleanup_workspace,
    copy_stable,
    identity,
    stable_file,
)
from evaluate.multiformat_native_unit_runtime import capture_native_observation
from evaluate.multiformat_native_unit_types import NativeUnitError, NativeUnitFailure
from evaluate.tests.multiformat_native_unit_fixture import (
    NativeUnitFixture,
    RecordingNativeRunner,
    make_native_unit_fixture,
)


class MultiFormatNativeUnitFinalRaceTests(unittest.TestCase):
    def test_execution_snapshot_race_is_rejected_before_real_open(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_unit_fixture(root)
            request = self._pdf_request(root, fixture)
            sentinel = root / "execution-sentinel"
            _ = sentinel.write_bytes(b"preserve-execution-sentinel")
            inserted = False
            original_open = os.open

            def race(
                path: Path,
                flags: int,
                mode: int = 0o777,
                *,
                dir_fd: int | None = None,
            ) -> int:
                nonlocal inserted
                if (
                    Path(path).name == "execution.json"
                    and flags & os.O_CREAT
                    and not inserted
                ):
                    inserted = True
                    _ = Path(path).symlink_to(sentinel)
                return original_open(path, flags, mode, dir_fd=dir_fd)

            with (
                patch("evaluate.multiformat_native_unit_io.os.open", side_effect=race),
                self.assertRaises(NativeUnitError) as raised,
            ):
                _ = capture_native_observation(request, RecordingNativeRunner())

            self.assertEqual(raised.exception.failure, NativeUnitFailure.OUTPUT_INVALID)
            self.assertEqual(sentinel.read_bytes(), b"preserve-execution-sentinel")
            self.assertTrue(inserted)
            self.assertFalse(request.observation_dir.exists())

    def test_cleanup_race_preserves_attacker_and_owned_inode(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_unit_fixture(root)
            request = fixture.request(root, DocumentFormat.PDF)
            workspace = root / "workspace"
            workspace.mkdir()
            _ = (workspace / "owned").write_bytes(b"owned")
            expected = identity(workspace.lstat())
            backup = root / "owned-backup"
            attacker_file = workspace / "attacker"
            original_rmtree = shutil.rmtree
            swapped = False

            def race(path: Path) -> None:
                nonlocal swapped
                if not swapped:
                    swapped = True
                    if workspace.exists():
                        _ = workspace.rename(backup)
                    workspace.mkdir()
                    _ = attacker_file.write_bytes(b"attacker")
                original_rmtree(path)

            with (
                patch(
                    "evaluate.multiformat_native_unit_files.shutil.rmtree",
                    side_effect=race,
                ),
                self.assertRaises(NativeUnitError) as raised,
            ):
                cleanup_workspace(workspace, expected, request)

            self.assertEqual(raised.exception.failure, NativeUnitFailure.OUTPUT_INVALID)
            self.assertTrue(swapped)
            self.assertTrue(attacker_file.is_file())
            self.assertEqual(attacker_file.read_bytes(), b"attacker")
            self.assertFalse(backup.exists())

    def test_cleanup_failure_does_not_publish_observation(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_unit_fixture(root)
            request = self._pdf_request(root, fixture)

            with (
                patch(
                    "evaluate.multiformat_native_unit_files.shutil.rmtree",
                    side_effect=OSError("cleanup failed"),
                ),
                self.assertRaises(NativeUnitError) as raised,
            ):
                _ = capture_native_observation(request, RecordingNativeRunner())

            self.assertEqual(raised.exception.failure, NativeUnitFailure.OUTPUT_INVALID)
            self.assertTrue(
                any(
                    "snapshot cleanup failed" in note
                    for note in raised.exception.__notes__
                )
            )
            self.assertFalse(request.observation_dir.exists())

    def test_destination_race_is_at_exclusive_open_boundary(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_unit_fixture(root)
            request = fixture.request(root, DocumentFormat.PDF)
            source = root / "source.bin"
            _ = source.write_bytes(b"source-bytes")
            expected = stable_file(source, request, NativeUnitFailure.SOURCE_INVALID)
            destination = root / "destination.bin"
            sentinel = root / "outside-sentinel"
            _ = sentinel.write_bytes(b"do-not-overwrite")
            inserted = False
            original_open = os.open

            def race(
                path: Path,
                flags: int,
                mode: int = 0o777,
                *,
                dir_fd: int | None = None,
            ) -> int:
                nonlocal inserted
                if path == destination and flags & os.O_CREAT and not inserted:
                    inserted = True
                    _ = destination.symlink_to(sentinel)
                return original_open(path, flags, mode, dir_fd=dir_fd)

            with (
                patch("evaluate.multiformat_native_unit_io.os.open", side_effect=race),
                self.assertRaises(NativeUnitError),
            ):
                _ = copy_stable(source, destination, expected, request)

            self.assertTrue(inserted)
            self.assertTrue(destination.is_symlink())
            self.assertEqual(sentinel.read_bytes(), b"do-not-overwrite")

    def test_source_substitution_cannot_be_followed_by_file_path(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_unit_fixture(root)
            request = self._pdf_request(root, fixture)
            source = request.source.path
            outside = root / "outside.pdf"
            _ = outside.write_bytes(b"%PDF-1.4\noutside\n")
            original_resolve = Path.resolve
            original_open = os.open
            resolve_raced = False
            open_raced = False

            def resolve_race(path: Path, strict: bool = False) -> Path:
                nonlocal resolve_raced
                if path == source and not resolve_raced:
                    resolve_raced = True
                    source.unlink()
                    _ = source.symlink_to(outside)
                return original_resolve(path, strict=strict)

            def open_race(
                path: Path,
                flags: int,
                mode: int = 0o777,
                *,
                dir_fd: int | None = None,
            ) -> int:
                nonlocal open_raced
                if Path(path).name == source.name and not open_raced:
                    open_raced = True
                    source.unlink()
                    _ = source.symlink_to(outside)
                return original_open(path, flags, mode, dir_fd=dir_fd)

            with (
                patch.object(Path, "resolve", resolve_race),
                patch(
                    "evaluate.multiformat_native_unit_io.os.open", side_effect=open_race
                ),
                self.assertRaises(NativeUnitError) as raised,
            ):
                _ = capture_native_observation(request, RecordingNativeRunner())

            self.assertEqual(raised.exception.failure, NativeUnitFailure.OUTPUT_INVALID)
            self.assertTrue(resolve_raced or open_raced)
            self.assertTrue(source.is_symlink())
            self.assertFalse(request.observation_dir.exists())

    @staticmethod
    def _pdf_request(root: Path, fixture: NativeUnitFixture):
        request = fixture.request(root, DocumentFormat.PDF)
        source = root / "source.pdf"
        _ = source.write_bytes(b"%PDF-1.4\nfixture\n")
        return replace(
            request,
            source=replace(
                request.source,
                path=source,
                relative_path="sources/source.pdf",
            ),
        )


if __name__ == "__main__":
    _ = unittest.main()
