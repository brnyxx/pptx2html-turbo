from __future__ import annotations

import platform
import tempfile
import unittest
from dataclasses import fields, replace
from pathlib import Path
from unittest.mock import patch

import evaluate.multiformat_native_unit_types as native_types
from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_native_unit_observation import (
    capture as one_observation_capture,
)
from evaluate.multiformat_native_unit_runtime import capture_native_observation
from evaluate.multiformat_native_unit_types import NativeUnitError, NativeUnitFailure
from evaluate.multiformat_schema import object_value, sha256_file
from evaluate.multiformat_strict_json import read_strict_object
from evaluate.tests.multiformat_native_unit_fixture import (
    RecordingNativeRunner,
    make_native_unit_fixture,
)


class MultiFormatNativeUnitContractTests(unittest.TestCase):
    def test_unsupported_platform_fails_before_tool_execution(self) -> None:
        for operating_system, architecture in (
            ("Windows", "arm64"),
            ("Darwin", "s390x"),
        ):
            with (
                self.subTest(
                    operating_system=operating_system, architecture=architecture
                ),
                tempfile.TemporaryDirectory() as temp_dir,
            ):
                root = Path(temp_dir)
                fixture = make_native_unit_fixture(root)
                runner = RecordingNativeRunner()
                request = fixture.request(root, DocumentFormat.PDF)
                request = replace(
                    request,
                    source=replace(request.source, path=root / "missing-source.pdf"),
                    runtime=replace(
                        request.runtime,
                        soffice=root / "missing-libreoffice",
                        pdfinfo=root / "missing-pdfinfo",
                    ),
                )
                with (
                    patch.object(platform, "system", return_value=operating_system),
                    patch.object(platform, "machine", return_value=architecture),
                    self.assertRaises(NativeUnitError) as raised,
                ):
                    _ = capture_native_observation(request, runner)
                self.assertEqual(
                    raised.exception.failure, NativeUnitFailure.UNSUPPORTED_PLATFORM
                )
                self.assertIsNone(raised.exception.document_format)
                self.assertIsNone(raised.exception.source_id)
                self.assertTrue(raised.exception.detail)
                self.assertEqual(runner.requests, [])

    def test_version_identity_uses_first_nonempty_ascii_trimmed_line(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_unit_fixture(root)
            runner = RecordingNativeRunner()
            runner.pdfinfo_version_output = (
                b"\n \t pdfinfo version 26.03.0 \t\nignored\n"
            )
            request = fixture.request(root, DocumentFormat.PDF)
            source = root / "source.pdf"
            _ = source.write_bytes(b"%PDF-1.4\nfixture\n")
            request = replace(
                request,
                source=replace(
                    request.source, path=source, relative_path="sources/source.pdf"
                ),
            )
            _ = capture_native_observation(request, runner)
            execution = read_strict_object(request.observation_dir / "execution.json")
            tools = object_value(execution, "tools")
            pdfinfo = object_value(tools, "pdfinfo")
            self.assertEqual(pdfinfo["version"], "pdfinfo version 26.03.0")
            self.assertIs(type(runner.requests[0].timeout_seconds), int)
            self.assertEqual(runner.requests[0].timeout_seconds, 120)

    def test_version_identity_rejects_nul_and_embedded_carriage_return(self) -> None:
        for output in (b"pdfinfo\x00version\n", b"pdfinfo 26\r03\n"):
            with self.subTest(output=output), tempfile.TemporaryDirectory() as temp_dir:
                root = Path(temp_dir)
                fixture = make_native_unit_fixture(root)
                runner = RecordingNativeRunner()
                runner.pdfinfo_version_output = output
                request = fixture.request(root, DocumentFormat.PDF)
                source = root / "source.pdf"
                _ = source.write_bytes(b"%PDF-1.4\nfixture\n")
                request = replace(
                    request,
                    source=replace(
                        request.source, path=source, relative_path="sources/source.pdf"
                    ),
                )
                with self.assertRaises(NativeUnitError) as raised:
                    _ = capture_native_observation(request, runner)
                self.assertEqual(
                    raised.exception.failure, NativeUnitFailure.OUTPUT_INVALID
                )

    def test_version_request_uses_literal_integer_timeout(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_unit_fixture(root)
            source = root / "source.pdf"
            _ = source.write_bytes(b"%PDF-1.4\nfixture\n")
            request = fixture.request(root, DocumentFormat.PDF)
            request = replace(
                request,
                source=replace(
                    request.source, path=source, relative_path="sources/source.pdf"
                ),
            )
            runner = RecordingNativeRunner()
            _ = capture_native_observation(request, runner)
            self.assertEqual(runner.requests[0].timeout_seconds, 120)
            self.assertIs(type(runner.requests[0].timeout_seconds), int)

    def test_schema_bindings_match_observation_parent_inputs_without_rewriting(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_unit_fixture(root)
            request = fixture.request(root, DocumentFormat.PDF)
            source = root / "source.pdf"
            _ = source.write_bytes(b"%PDF-1.4\nfixture\n")
            request = replace(
                request,
                source=replace(
                    request.source, path=source, relative_path="sources/source.pdf"
                ),
            )
            observation = capture_native_observation(request, RecordingNativeRunner())
            execution = read_strict_object(request.observation_dir / "execution.json")
            source_record = object_value(execution, "source")
            evidence = object_value(execution, "evidence")
            reference = object_value(evidence, "reference_pdf")
            pdfinfo = object_value(evidence, "pdfinfo")
            observation_root = (
                f"observations/{request.source.document_format.value}/"
                f"{request.source.source_id}/run-{request.run}"
            )

            self.assertEqual(source_record["id"], observation.source.source_id)
            self.assertEqual(
                source_record["format"], observation.source.document_format.value
            )
            self.assertEqual(source_record["path"], observation.source.relative_path)
            self.assertEqual(
                source_record["sha256"], sha256_file(observation.source.path)
            )
            self.assertEqual(execution["run"], observation.run)
            self.assertEqual(execution["workspace_nonce"], observation.workspace_nonce)
            self.assertEqual(execution["unit_count"], observation.unit_count)
            self.assertEqual(reference["path"], f"{observation_root}/reference.pdf")
            self.assertEqual(reference["sha256"], observation.reference_pdf_sha256)
            self.assertEqual(pdfinfo["path"], f"{observation_root}/pdfinfo.txt")
            self.assertEqual(pdfinfo["sha256"], observation.pdfinfo_sha256)

    def test_public_native_types_have_no_summary_and_error_fields_are_exact(
        self,
    ) -> None:
        self.assertEqual(
            one_observation_capture.__module__,
            "evaluate.multiformat_native_unit_observation",
        )
        self.assertNotIn("NativeUnitSummary", vars(native_types))
        self.assertEqual(
            tuple(field.name for field in fields(native_types.NativeUnitError)),
            ("failure", "document_format", "source_id", "detail"),
        )
        self.assertEqual(
            native_types.NativeUnitFailure.UNSUPPORTED_PLATFORM.value,
            "unsupported-platform",
        )


if __name__ == "__main__":
    _ = unittest.main()
