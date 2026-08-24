from __future__ import annotations

import hashlib
import json
import tempfile
import unittest
from dataclasses import replace
from pathlib import Path

from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_native_unit_runtime import capture_native_observation
from evaluate.multiformat_native_unit_types import NativeUnitError, NativeUnitFailure
from evaluate.multiformat_strict_json import read_strict_object
from evaluate.tests.multiformat_native_unit_fixture import (
    RecordingNativeRunner,
    make_native_unit_fixture,
)

OFFICE_FORMATS = (
    DocumentFormat.DOC,
    DocumentFormat.DOCX,
    DocumentFormat.XLS,
    DocumentFormat.XLSX,
    DocumentFormat.PPT,
    DocumentFormat.PPTX,
)


class MultiFormatNativeUnitRuntimeTests(unittest.TestCase):
    def test_pdf_copies_source_and_invokes_only_pdfinfo(self) -> None:
        # Given
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_unit_fixture(root)
            source = root / "source.pdf"
            _ = source.write_bytes(b"%PDF-1.4\nfixture-pdf\n")
            original_request = fixture.request(root, DocumentFormat.PDF)
            request = replace(
                original_request,
                source=replace(
                    original_request.source,
                    path=source,
                    relative_path="sources/source.pdf",
                ),
            )
            runner = RecordingNativeRunner()

            # When
            observation = capture_native_observation(request, runner)

            # Then
            self.assertEqual(observation.unit_count, 1)
            commands = [item.command for item in runner.requests]
            self.assertEqual(len(commands), 2)
            self.assertEqual(commands[0], (fixture.pdfinfo.resolve().as_posix(), "-v"))
            self.assertEqual(commands[1][0], fixture.pdfinfo.resolve().as_posix())
            self.assertTrue(commands[1][-1].endswith("/source.pdf"))
            self.assertEqual(
                {path.name for path in request.observation_dir.iterdir()},
                {"execution.json", "reference.pdf", "pdfinfo.txt"},
            )
            self.assertEqual(
                (request.observation_dir / "reference.pdf").read_bytes(),
                source.read_bytes(),
            )

    def test_office_formats_use_exact_locked_arguments(self) -> None:
        # Given
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_unit_fixture(root)
            for document_format in OFFICE_FORMATS:
                with self.subTest(document_format=document_format.value):
                    runner = RecordingNativeRunner()
                    request = fixture.request(root, document_format)

                    # When
                    _ = capture_native_observation(request, runner)

                    # Then
                    conversion = next(
                        item
                        for item in runner.requests
                        if "--convert-to" in item.command
                    )
                    source = Path(conversion.command[-1])
                    output_dir = Path(
                        conversion.command[conversion.command.index("--outdir") + 1]
                    )
                    self.assertEqual(
                        conversion.command[:8],
                        (
                            fixture.soffice.resolve().as_posix(),
                            "--headless",
                            "--nologo",
                            "--nodefault",
                            "--nolockcheck",
                            "--nofirststartwizard",
                            conversion.command[6],
                            "--convert-to",
                        ),
                    )
                    self.assertTrue(
                        conversion.command[6].startswith(
                            "-env:UserInstallation=file://"
                        )
                    )
                    self.assertEqual(conversion.command[8], "pdf")
                    self.assertEqual(conversion.command[9], "--outdir")
                    self.assertEqual(output_dir.name, "export")
                    self.assertEqual(source.suffix, f".{document_format.value}")
                    metadata = runner.requests[-1]
                    self.assertEqual(
                        metadata.command[0], fixture.pdfinfo.resolve().as_posix()
                    )
                    self.assertEqual(
                        metadata.command[-1],
                        (output_dir / "source.pdf").as_posix(),
                    )

    def test_execution_retains_hashes_without_workspace_paths(self) -> None:
        # Given
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_unit_fixture(root)
            runner = RecordingNativeRunner()
            request = fixture.request(root, DocumentFormat.DOCX)
            runner.stdout_output = b"conversion stdout"
            runner.stderr_output = b"conversion stderr"

            # When
            _ = capture_native_observation(request, runner)

            # Then
            execution = read_strict_object(request.observation_dir / "execution.json")
            evidence = execution.get("evidence")
            assert isinstance(evidence, dict)
            reference_pdf = evidence.get("reference_pdf")
            pdfinfo = evidence.get("pdfinfo")
            assert isinstance(reference_pdf, dict)
            assert isinstance(pdfinfo, dict)
            reference_hash = reference_pdf.get("sha256")
            pdfinfo_hash = pdfinfo.get("sha256")
            logs = execution.get("logs")
            assert isinstance(reference_hash, str)
            assert isinstance(pdfinfo_hash, str)
            assert isinstance(logs, list)
            log_hashes = {
                value
                for log in logs
                if isinstance(log, dict)
                for field in ("stdout_sha256", "stderr_sha256")
                if isinstance(value := log.get(field), str)
            }
            self.assertIn(
                hashlib.sha256(b"conversion stdout").hexdigest(),
                log_hashes,
            )
            self.assertIn(
                hashlib.sha256(b"conversion stderr").hexdigest(),
                log_hashes,
            )
            serialized = json.dumps(execution, sort_keys=True)
            self.assertNotIn(request.observation_dir.parent.as_posix(), serialized)
            self.assertNotIn(".native-unit-", serialized)
            self.assertRegex(
                reference_hash,
                r"^[0-9a-f]{64}$",
            )
            self.assertRegex(
                pdfinfo_hash,
                r"^[0-9a-f]{64}$",
            )
            self.assertEqual(
                {path.name for path in request.observation_dir.iterdir()},
                {"execution.json", "reference.pdf", "pdfinfo.txt"},
            )

    def test_process_requests_have_unique_isolated_roots_and_clean_environment(
        self,
    ) -> None:
        # Given
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_unit_fixture(root)
            first_runner = RecordingNativeRunner()
            second_runner = RecordingNativeRunner()

            # When
            _ = capture_native_observation(
                fixture.request(root, DocumentFormat.DOCX, run=1), first_runner
            )
            _ = capture_native_observation(
                fixture.request(root, DocumentFormat.DOCX, run=2), second_runner
            )

            # Then
            first = first_runner.requests[2]
            second = second_runner.requests[2]
            self.assertNotEqual(
                dict(first.environment)["HOME"], dict(second.environment)["HOME"]
            )
            self.assertNotEqual(
                dict(first.environment)["TMPDIR"], dict(second.environment)["TMPDIR"]
            )
            self.assertEqual(
                set(dict(first.environment)),
                {"FONTCONFIG_FILE", "HOME", "LANG", "LC_ALL", "PATH", "TMPDIR", "TZ"},
            )
            self.assertEqual(dict(first.environment)["LANG"], "C")
            self.assertEqual(dict(first.environment)["LC_ALL"], "C")
            self.assertEqual(dict(first.environment)["TZ"], "UTC")

    def test_nonzero_timeout_missing_malformed_and_oversize_are_typed_failures(
        self,
    ) -> None:
        cases = (
            ("nonzero", NativeUnitFailure.PROCESS_FAILED, 7, None, True),
            ("timeout", NativeUnitFailure.TIMEOUT, 0, "converter timeout", True),
            ("missing", NativeUnitFailure.OUTPUT_MISSING, 0, None, False),
            ("malformed", NativeUnitFailure.PAGES_MALFORMED, 0, None, True),
            ("oversize", NativeUnitFailure.LOG_OVERSIZE, 0, None, True),
        )
        for name, failure, exit_code, runner_failure, write_pdf in cases:
            with self.subTest(name=name), tempfile.TemporaryDirectory() as temp_dir:
                root = Path(temp_dir)
                fixture = make_native_unit_fixture(root)
                runner = RecordingNativeRunner()
                runner.exit_code = exit_code
                runner.failure = runner_failure
                runner.write_pdf = write_pdf
                if name == "malformed":
                    runner.pdfinfo_output = b"Producer: fixture\n"
                if name == "oversize":
                    runner.stdout_output = b"x" * (1024 * 1024 + 1)
                request = fixture.request(root, DocumentFormat.DOCX)

                # When / Then
                with self.assertRaises(NativeUnitError) as context:
                    _ = capture_native_observation(request, runner)
                self.assertEqual(context.exception.failure, failure)
                self.assertEqual(
                    context.exception.source_id,
                    request.source.source_id,
                )
                self.assertEqual(
                    context.exception.document_format,
                    request.source.document_format,
                )
                self.assertFalse(request.observation_dir.exists())

    def test_zero_and_multiple_pages_fail_closed(self) -> None:
        for output in (b"Pages:           0\n", b"Pages:           1\nPages: 2\n"):
            with self.subTest(output=output), tempfile.TemporaryDirectory() as temp_dir:
                root = Path(temp_dir)
                fixture = make_native_unit_fixture(root)
                runner = RecordingNativeRunner()
                runner.pdfinfo_output = output

                # When / Then
                with self.assertRaises(NativeUnitError) as context:
                    _ = capture_native_observation(
                        fixture.request(root, DocumentFormat.PDF), runner
                    )
                self.assertEqual(
                    context.exception.failure,
                    NativeUnitFailure.PAGES_MALFORMED,
                )


if __name__ == "__main__":
    _ = unittest.main()
