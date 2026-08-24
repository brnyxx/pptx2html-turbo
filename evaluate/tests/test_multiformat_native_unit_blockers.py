from __future__ import annotations

import os
import tempfile
import unittest
from dataclasses import replace
from pathlib import Path
from typing import IO
from unittest.mock import patch

from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_native_unit_files import (
    cleanup_workspace,
    copy_stable,
    identity,
    stable_file,
)
from evaluate.multiformat_native_unit_process import pages as parse_pages
from evaluate.multiformat_native_unit_runtime import capture_native_observation
from evaluate.multiformat_native_unit_types import (
    NativeUnitError,
    NativeUnitFailure,
    NativeUnitRequest,
)
from evaluate.tests.multiformat_native_unit_fixture import (
    NativeUnitFixture,
    RecordingNativeRunner,
    make_native_unit_fixture,
)


class PrimaryFailure(Exception):
    pass


class MultiFormatNativeUnitBlockerTests(unittest.TestCase):
    def test_cleanup_replacement_is_preserved_and_typed(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_unit_fixture(root)
            request = fixture.request(root, DocumentFormat.PDF)
            workspace = root / "workspace"
            workspace.mkdir()
            expected = identity(workspace.lstat())
            sentinel = root / "sentinel"
            _ = sentinel.write_text("keep", encoding="utf-8")
            workspace.rmdir()
            workspace.symlink_to(sentinel)

            with self.assertRaises(NativeUnitError) as raised:
                cleanup_workspace(workspace, expected, request)
            self.assertEqual(raised.exception.failure, NativeUnitFailure.OUTPUT_INVALID)
            self.assertTrue(workspace.is_symlink())
            self.assertEqual(sentinel.read_text(encoding="utf-8"), "keep")

    def test_cleanup_missing_workspace_is_not_success(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_unit_fixture(root)
            request = fixture.request(root, DocumentFormat.PDF)
            workspace = root / "workspace"
            workspace.mkdir()
            expected = identity(workspace.lstat())
            workspace.rmdir()

            with self.assertRaises(NativeUnitError):
                cleanup_workspace(workspace, expected, request)

    def test_cleanup_error_is_typed_and_primary_error_gets_a_note(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_unit_fixture(root)
            request = fixture.request(root, DocumentFormat.PDF)
            workspace = root / "workspace"
            workspace.mkdir()
            expected = identity(workspace.lstat())
            with (
                patch(
                    "evaluate.multiformat_native_unit_files.shutil.rmtree",
                    side_effect=OSError("cleanup failed"),
                ),
                self.assertRaises(NativeUnitError),
            ):
                cleanup_workspace(workspace, expected, request)

            workspace = root / "primary-workspace"
            workspace.mkdir()
            expected = identity(workspace.lstat())
            with (
                patch(
                    "evaluate.multiformat_native_unit_files.shutil.rmtree",
                    side_effect=OSError("cleanup failed"),
                ),
                self.assertRaises(PrimaryFailure) as raised,
            ):
                try:
                    raise PrimaryFailure("primary")
                finally:
                    cleanup_workspace(workspace, expected, request)
            self.assertTrue(
                any("cleanup failed" in note for note in raised.exception.__notes__)
            )

    def test_copy_stable_rejects_final_boundary_symlink_without_overwrite(self) -> None:
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
            original_lexists = os.path.lexists

            def race(path: Path) -> bool:
                nonlocal inserted
                if path == destination and not inserted:
                    inserted = True
                    destination.symlink_to(sentinel)
                    return False
                return original_lexists(path)

            with (
                patch(
                    "evaluate.multiformat_native_unit_files.os.path.lexists",
                    side_effect=race,
                ),
                self.assertRaises(NativeUnitError),
            ):
                _ = copy_stable(source, destination, expected, request)
            self.assertEqual(sentinel.read_bytes(), b"do-not-overwrite")

    def test_metadata_is_parsed_and_retained_from_one_stable_snapshot(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_unit_fixture(root)
            request = self._pdf_request(root, fixture)
            runner = RecordingNativeRunner()
            original_pages = parse_pages

            def parse_then_mutate(
                value: bytes | Path, current_request: NativeUnitRequest
            ) -> int:
                count = original_pages(value, current_request)
                _ = runner.requests[-1].stdout_path.write_bytes(b"Pages:           2\n")
                return count

            with patch(
                "evaluate.multiformat_native_unit_observation.pages",
                side_effect=parse_then_mutate,
            ):
                observation = capture_native_observation(request, runner)
            self.assertEqual(observation.unit_count, 1)
            self.assertEqual(
                (request.observation_dir / "pdfinfo.txt").read_bytes(),
                b"Pages:           1\n",
            )

    def test_final_path_open_failure_cannot_leave_published_observation(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_unit_fixture(root)
            request = self._pdf_request(root, fixture)
            original_open = Path.open

            def reject_published(
                path: Path,
                mode: str = "r",
                buffering: int = -1,
                encoding: str | None = None,
                errors: str | None = None,
                newline: str | None = None,
            ) -> IO[str] | IO[bytes]:
                if path.parent == request.observation_dir:
                    raise OSError("post-publication read")
                return original_open(path, mode, buffering, encoding, errors, newline)

            with patch.object(Path, "open", reject_published):
                observation = capture_native_observation(
                    request, RecordingNativeRunner()
                )
            self.assertTrue(observation.observation_dir.exists())
            self.assertEqual(
                {item.name for item in observation.observation_dir.iterdir()},
                {"execution.json", "reference.pdf", "pdfinfo.txt"},
            )

    def test_boolean_run_and_exit_code_are_rejected_at_runtime_boundary(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_unit_fixture(root)
            request = replace(self._pdf_request(root, fixture), run=True)
            with self.assertRaises(NativeUnitError):
                _ = capture_native_observation(request, RecordingNativeRunner())
            request = self._pdf_request(root, fixture)
            runner = RecordingNativeRunner()
            runner.exit_code = False
            with self.assertRaises(NativeUnitError) as raised:
                _ = capture_native_observation(request, runner)
            self.assertEqual(raised.exception.failure, NativeUnitFailure.PROCESS_FAILED)

    def test_missing_stdout_or_stderr_is_typed(self) -> None:
        for field in ("missing_stdout", "missing_stderr"):
            with self.subTest(field=field), tempfile.TemporaryDirectory() as temp_dir:
                root = Path(temp_dir)
                fixture = make_native_unit_fixture(root)
                request = self._pdf_request(root, fixture)
                runner = RecordingNativeRunner()
                setattr(runner, field, True)
                with self.assertRaises(NativeUnitError) as raised:
                    _ = capture_native_observation(request, runner)
                self.assertEqual(
                    raised.exception.failure, NativeUnitFailure.OUTPUT_INVALID
                )

    def test_executable_identity_drift_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_unit_fixture(root)
            request = self._pdf_request(root, fixture)
            runner = RecordingNativeRunner()
            runner.mutate_tool = fixture.pdfinfo.resolve()
            with self.assertRaises(NativeUnitError) as raised:
                _ = capture_native_observation(request, runner)
            self.assertEqual(raised.exception.failure, NativeUnitFailure.OUTPUT_INVALID)
            self.assertFalse(request.observation_dir.exists())

    @staticmethod
    def _pdf_request(root: Path, fixture: NativeUnitFixture) -> NativeUnitRequest:
        request = fixture.request(root, DocumentFormat.PDF)
        source = root / "source.pdf"
        _ = source.write_bytes(b"%PDF-1.4\nfixture\n")
        return replace(
            request,
            source=replace(
                request.source, path=source, relative_path="sources/source.pdf"
            ),
        )


if __name__ == "__main__":
    _ = unittest.main()
