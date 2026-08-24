from __future__ import annotations

import os
import tempfile
import threading
import unittest
from pathlib import Path
from unittest.mock import patch

from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_native_unit_files import output_file, stable_file
from evaluate.multiformat_native_unit_io import read_descriptor, write_new
from evaluate.multiformat_native_unit_runtime import capture_native_observation
from evaluate.multiformat_native_unit_types import (
    NativeExecutionData,
    NativeStableFile,
    NativeUnitError,
    NativeUnitFailure,
    NativeUnitRequest,
    execution_record,
)
from evaluate.tests.multiformat_native_unit_fixture import (
    RecordingNativeRunner,
    make_native_unit_fixture,
)


class MultiFormatNativeUnitStabilityTests(unittest.TestCase):
    def test_same_size_mtime_restored_ctime_mutation_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_unit_fixture(root)
            request = fixture.request(root, DocumentFormat.PDF)
            source = root / "source.bin"
            _ = source.write_bytes(b"stable-bytes")
            before = source.stat()

            def mutate(descriptor: int) -> bytes:
                content = read_descriptor(descriptor)
                _ = os.lseek(descriptor, 0, os.SEEK_SET)
                _ = os.write(descriptor, b"X" + content[1:])
                _ = os.utime(
                    source,
                    ns=(before.st_atime_ns, before.st_mtime_ns),
                )
                return content

            with (
                patch(
                    "evaluate.multiformat_native_unit_files.read_descriptor",
                    side_effect=mutate,
                ),
                self.assertRaises(NativeUnitError) as raised,
            ):
                _ = stable_file(source, request, NativeUnitFailure.SOURCE_INVALID)

            self.assertEqual(raised.exception.failure, NativeUnitFailure.OUTPUT_INVALID)

    def test_pdf_header_is_validated_from_stable_bytes(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_unit_fixture(root)
            request = fixture.request(root, DocumentFormat.PDF)
            source = root / "source.pdf"
            _ = source.write_bytes(b"%PDF-1.4\nvalid\n")
            stable = stable_file(source, request, NativeUnitFailure.OUTPUT_MISSING)

            with (
                patch(
                    "evaluate.multiformat_native_unit_files.stable_bytes",
                    return_value=(stable, b"not a PDF"),
                ),
                self.assertRaises(NativeUnitError) as raised,
            ):
                _ = output_file(source, request)

            self.assertEqual(raised.exception.failure, NativeUnitFailure.OUTPUT_INVALID)

    def test_evidence_mutation_cannot_publish_mismatched_hashes(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_unit_fixture(root)
            request = fixture.request(root, DocumentFormat.PDF)
            reference_path: Path | None = None
            original_write_new = write_new

            def remember_reference(
                destination: Path,
                content: bytes,
                current_request: NativeUnitRequest,
            ) -> NativeStableFile:
                nonlocal reference_path
                result = original_write_new(destination, content, current_request)
                if destination.name == "reference.pdf":
                    reference_path = destination
                return result

            def mutate(data: NativeExecutionData):
                if reference_path is None:
                    raise AssertionError("reference was not retained")
                _ = reference_path.write_bytes(b"mutated-reference")
                return execution_record(data)

            with (
                patch(
                    "evaluate.multiformat_native_unit_files.write_new",
                    side_effect=remember_reference,
                ),
                patch(
                    "evaluate.multiformat_native_unit_observation.execution_record",
                    side_effect=mutate,
                ),
                self.assertRaises(NativeUnitError) as raised,
            ):
                _ = capture_native_observation(request, RecordingNativeRunner())

            self.assertEqual(raised.exception.failure, NativeUnitFailure.OUTPUT_INVALID)
            self.assertFalse(request.observation_dir.exists())

    def test_zero_byte_write_is_typed_instead_of_spinning(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_unit_fixture(root)
            request = fixture.request(root, DocumentFormat.PDF)
            destination = root / "destination.bin"
            finished = threading.Event()
            failures: list[NativeUnitError] = []

            def invoke() -> None:
                try:
                    _ = write_new(destination, b"content", request)
                except NativeUnitError as error:
                    failures.append(error)
                finally:
                    finished.set()

            with patch("evaluate.multiformat_native_unit_io.os.write", return_value=0):
                thread = threading.Thread(target=invoke, daemon=True)
                thread.start()
                self.assertTrue(finished.wait(timeout=1.0))

            self.assertEqual(len(failures), 1)
            self.assertIsInstance(failures[0], NativeUnitError)
            self.assertEqual(
                failures[0].failure,
                NativeUnitFailure.OUTPUT_INVALID,
            )


if __name__ == "__main__":
    _ = unittest.main()
