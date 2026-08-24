from __future__ import annotations

import json
import sys
import tempfile
import unittest
from dataclasses import replace
from pathlib import Path
from unittest.mock import patch

from evaluate.multiformat_candidate_process import CandidateProcessFailure
from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_native_unit_runtime import capture_native_observation
from evaluate.multiformat_native_unit_types import NativeUnitError, NativeUnitFailure
from evaluate.multiformat_strict_json import read_strict_object
from evaluate.tests.multiformat_native_unit_fixture import (
    RecordingNativeRunner,
    make_native_unit_fixture,
)

OFFICE_FORMATS = tuple(
    format for format in DocumentFormat if format is not DocumentFormat.PDF
)


class MultiFormatNativeUnitRuntimeTests(unittest.TestCase):
    def test_pdf_route_needs_only_pdfinfo_and_retains_exact_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_unit_fixture(root)
            source = root / "source.pdf"
            _ = source.write_bytes(b"%PDF-1.4\nfixture-pdf\n")
            base = fixture.request(root, DocumentFormat.PDF)
            request = replace(
                base,
                source=replace(
                    base.source, path=source, relative_path="sources/source.pdf"
                ),
                runtime=replace(
                    base.runtime,
                    soffice=root / "missing-soffice",
                    font_bundle=root / "missing-fonts.json",
                ),
            )
            runner = RecordingNativeRunner()

            observation = capture_native_observation(request, runner)
            commands = [item.command for item in runner.requests]
            execution = read_strict_object(request.observation_dir / "execution.json")

            self.assertEqual(observation.unit_count, 1)
            self.assertEqual(observation.workspace_nonce, request.nonce)
            self.assertEqual(execution["workspace_nonce"], request.nonce)
            self.assertEqual(len(commands), 2)
            self.assertEqual(commands[0], (fixture.pdfinfo.resolve().as_posix(), "-v"))
            self.assertTrue(commands[1][-1].endswith("/source.pdf"))
            self.assertEqual(
                {path.name for path in request.observation_dir.iterdir()},
                {"execution.json", "reference.pdf", "pdfinfo.txt"},
            )
            self.assertEqual(
                (request.observation_dir / "reference.pdf").read_bytes(),
                source.read_bytes(),
            )
            self.assertRegex(observation.reference_pdf_sha256, r"^[0-9a-f]{64}$")
            self.assertNotIn(root.as_posix(), json.dumps(execution, sort_keys=True))

    def test_office_formats_use_exact_locked_arguments(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_unit_fixture(root)
            for document_format in OFFICE_FORMATS:
                with self.subTest(document_format=document_format.value):
                    runner = RecordingNativeRunner()
                    request = fixture.request(root, document_format)
                    _ = capture_native_observation(request, runner)
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
                    self.assertEqual(conversion.command[8:10], ("pdf", "--outdir"))
                    self.assertEqual(output_dir.name, "export")
                    self.assertEqual(source.suffix, f".{document_format.value}")

    def test_process_roots_are_unique_and_environment_is_clean(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_unit_fixture(root)
            first = RecordingNativeRunner()
            second = RecordingNativeRunner()
            _ = capture_native_observation(
                fixture.request(root, DocumentFormat.DOCX, run=1), first
            )
            _ = capture_native_observation(
                fixture.request(root, DocumentFormat.DOCX, run=2), second
            )
            first_env = dict(first.requests[2].environment)
            second_env = dict(second.requests[2].environment)
            self.assertNotEqual(first_env["HOME"], second_env["HOME"])
            self.assertNotEqual(first_env["TMPDIR"], second_env["TMPDIR"])
            self.assertEqual(
                set(first_env),
                {"FONTCONFIG_FILE", "HOME", "LANG", "LC_ALL", "PATH", "TMPDIR", "TZ"},
            )
            self.assertEqual(first_env["LANG"], "en_US.UTF-8")
            self.assertEqual(first_env["LC_ALL"], "en_US.UTF-8")
            self.assertEqual(first_env["TZ"], "UTC")

    def test_failures_are_typed_and_cleanup_is_complete(self) -> None:
        cases = (
            ("nonzero", NativeUnitFailure.PROCESS_FAILED, 7, None, True),
            (
                "timeout",
                NativeUnitFailure.TIMEOUT,
                0,
                CandidateProcessFailure.TIMEOUT,
                True,
            ),
            ("missing", NativeUnitFailure.OUTPUT_MISSING, 0, None, False),
            ("malformed-pages", NativeUnitFailure.PAGES_MALFORMED, 0, None, True),
            ("malformed-pdf", NativeUnitFailure.OUTPUT_INVALID, 0, None, True),
            ("oversize-log", NativeUnitFailure.LOG_OVERSIZE, 0, None, True),
        )
        for name, failure, exit_code, runner_failure, write_pdf in cases:
            with self.subTest(name=name), tempfile.TemporaryDirectory() as temp_dir:
                root = Path(temp_dir)
                fixture = make_native_unit_fixture(root)
                runner = RecordingNativeRunner()
                runner.exit_code, runner.failure, runner.write_pdf = (
                    exit_code,
                    runner_failure,
                    write_pdf,
                )
                if name == "malformed-pages":
                    runner.pdfinfo_output = b"Producer: fixture\n"
                if name == "malformed-pdf":
                    runner.pdf_output = b"not a PDF"
                if name == "oversize-log":
                    runner.stdout_output = b"x" * (1024 * 1024 + 1)
                request = fixture.request(root, DocumentFormat.DOCX)
                with self.assertRaises(NativeUnitError) as raised:
                    _ = capture_native_observation(request, runner)
                self.assertEqual(raised.exception.failure, failure)
                self.assertFalse(request.observation_dir.exists())

    def test_output_mutation_after_pdfinfo_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_unit_fixture(root)
            runner = RecordingNativeRunner()
            runner.mutate_pdf = True
            with self.assertRaises(NativeUnitError) as raised:
                _ = capture_native_observation(
                    fixture.request(root, DocumentFormat.DOCX), runner
                )
            self.assertEqual(raised.exception.failure, NativeUnitFailure.OUTPUT_INVALID)

    def test_nonce_must_be_exact_lowercase_hex_and_is_not_generated(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_unit_fixture(root)
            request = replace(fixture.request(root, DocumentFormat.PDF), nonce="A" * 64)
            with self.assertRaises(NativeUnitError) as raised:
                _ = capture_native_observation(request, RecordingNativeRunner())
            self.assertEqual(raised.exception.failure, NativeUnitFailure.SOURCE_INVALID)

    def test_unsupported_platform_fails_before_tool_execution(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_unit_fixture(root)
            runner = RecordingNativeRunner()
            with (
                patch.object(sys, "platform", "win32"),
                self.assertRaises(NativeUnitError) as raised,
            ):
                _ = capture_native_observation(
                    fixture.request(root, DocumentFormat.PDF), runner
                )
            self.assertEqual(
                raised.exception.failure, NativeUnitFailure.UNSUPPORTED_PLATFORM
            )
            self.assertEqual(runner.requests, [])

    def test_zero_and_multiple_pages_fail_closed(self) -> None:
        for output in (b"Pages:           0\n", b"Pages:           1\nPages: 2\n"):
            with self.subTest(output=output), tempfile.TemporaryDirectory() as temp_dir:
                root = Path(temp_dir)
                fixture = make_native_unit_fixture(root)
                source = root / "source.pdf"
                _ = source.write_bytes(b"%PDF-1.4\nfixture\n")
                base = fixture.request(root, DocumentFormat.PDF)
                request = replace(
                    base,
                    source=replace(
                        base.source, path=source, relative_path="sources/source.pdf"
                    ),
                )
                runner = RecordingNativeRunner()
                runner.pdfinfo_output = output
                with self.assertRaises(NativeUnitError) as raised:
                    _ = capture_native_observation(request, runner)
                self.assertEqual(
                    raised.exception.failure, NativeUnitFailure.PAGES_MALFORMED
                )


if __name__ == "__main__":
    _ = unittest.main()
