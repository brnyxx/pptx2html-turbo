from __future__ import annotations

import tempfile
import unittest
from dataclasses import replace
from pathlib import Path

from evaluate.multiformat_candidate_process import CandidateProcessFailure
from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_native_unit_cache import NativeObservationCache
from evaluate.multiformat_native_unit_files import font_environment, stable_file
from evaluate.multiformat_native_unit_types import (
    NativeCaptureTool,
    NativeCaptureTools,
    NativeUnitError,
    NativeUnitFailure,
    NativeVersionProbe,
)
from evaluate.tests.multiformat_native_unit_fixture import (
    RecordingNativeRunner,
    make_native_unit_fixture,
)


class MultiFormatNativeUnitCacheTests(unittest.TestCase):
    def test_second_capture_materializes_hit_without_processes(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_unit_fixture(root)
            request = fixture.request(root / "first", DocumentFormat.DOCX)
            source = stable_file(
                fixture.source,
                request,
                NativeUnitFailure.SOURCE_INVALID,
            )
            tools = NativeCaptureTools(
                NativeCaptureTool(
                    stable_file(
                        fixture.soffice,
                        request,
                        NativeUnitFailure.TOOL_MISSING,
                    ),
                    "LibreOffice 26.2.2.2",
                    NativeVersionProbe(
                        "libreoffice_version",
                        ("--version",),
                        120,
                        0,
                    ),
                ),
                NativeCaptureTool(
                    stable_file(
                        fixture.pdfinfo,
                        request,
                        NativeUnitFailure.TOOL_MISSING,
                    ),
                    "pdfinfo version 26.03.0",
                    NativeVersionProbe("pdfinfo_version", ("-v",), 120, 0),
                ),
            )
            prepared = replace(
                request,
                runtime=replace(request.runtime, tools=tools),
            )
            font_workspace = root / "font-workspace"
            font_workspace.mkdir()
            font = font_environment(prepared, font_workspace)
            cache = NativeObservationCache(
                root / "cache",
                "1" * 64,
                "2" * 64,
                font.environment_sha256,
                tools,
            )
            first_runner = RecordingNativeRunner()

            first = cache.capture(prepared, source.sha256, first_runner)

            second_runner = RecordingNativeRunner()
            second_runner.failure = CandidateProcessFailure.PIPES_UNAVAILABLE
            second = cache.capture(
                replace(
                    prepared,
                    observation_dir=root / "second",
                    nonce="f" * 64,
                ),
                source.sha256,
                second_runner,
            )

            self.assertEqual(second_runner.requests, [])
            self.assertEqual(first.workspace_nonce, second.workspace_nonce)
            self.assertEqual(first.execution_sha256, second.execution_sha256)
            self.assertEqual(first.reference_pdf_sha256, second.reference_pdf_sha256)
            self.assertEqual(first.pdfinfo_sha256, second.pdfinfo_sha256)
            self.assertTrue((root / "second" / "execution.json").is_file())
            entry = next((root / "cache" / "v1").glob("*/*"))
            _ = (entry / "reference.pdf").write_bytes(b"tampered")
            rejected_runner = RecordingNativeRunner()
            with self.assertRaises(NativeUnitError):
                _ = cache.capture(
                    replace(
                        prepared,
                        observation_dir=root / "rejected",
                        nonce="e" * 64,
                    ),
                    source.sha256,
                    rejected_runner,
                )
            self.assertEqual(rejected_runner.requests, [])
            self.assertFalse((root / "rejected").exists())


if __name__ == "__main__":
    _ = unittest.main()
