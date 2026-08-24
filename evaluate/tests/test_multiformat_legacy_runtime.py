from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_legacy_runtime import (
    LegacyExternalTools,
    LegacyProcessRequest,
    build_legacy_runtime,
)
from evaluate.multiformat_legacy_types import (
    LegacyConformanceError,
    LegacyPairJob,
)
from evaluate.multiformat_schema import sha256_file
from evaluate.tests.multiformat_source_fixture import write_positive_source


class MultiFormatLegacyRuntimeTests(unittest.TestCase):
    def test_uses_locked_filters_profiles_and_pdf_unit_check(self) -> None:
        # Given
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            tools = self._tools(root)
            requests: list[LegacyProcessRequest] = []
            runtime = build_legacy_runtime(
                tools,
                runner=lambda request: self._run(requests, request),
            )

            # When
            for document_format in (
                DocumentFormat.DOC,
                DocumentFormat.XLS,
                DocumentFormat.PPT,
            ):
                source = root / f"source.{self._modern(document_format)}"
                write_positive_source(
                    source,
                    self._modern(document_format),
                    document_format.value,
                )
                destination = root / f"result.{document_format.value}"
                count = runtime.materialize(
                    LegacyPairJob(
                        document_format.value,
                        document_format,
                        source,
                        destination,
                        root / f"work-{document_format.value}",
                    )
                )
                self.assertEqual(count, 1)
                self.assertTrue(destination.is_file())

            # Then
            self.assertEqual(
                [request.command for request in requests[:2]],
                [
                    (tools.soffice.resolve().as_posix(), "--version"),
                    (tools.pdfinfo.resolve().as_posix(), "-v"),
                ],
            )
            filters = [
                request.command[request.command.index("--convert-to") + 1]
                for request in requests
                if "--convert-to" in request.command
            ]
            self.assertEqual(
                filters,
                [
                    "doc:MS Word 97",
                    "pdf",
                    "xls:MS Excel 97",
                    "pdf",
                    "ppt:MS PowerPoint 97",
                    "pdf",
                ],
            )
            conversion_requests = [
                request for request in requests if "--convert-to" in request.command
            ]
            self.assertTrue(
                all(
                    any(
                        argument.startswith("-env:UserInstallation=file://")
                        for argument in request.command
                    )
                    for request in conversion_requests
                )
            )
            self.assertTrue(
                all(request.environment["TZ"] == "UTC" for request in requests)
            )
            self.assertEqual(runtime.tools.soffice_sha256, sha256_file(tools.soffice))
            self.assertEqual(runtime.tools.pdfinfo_sha256, sha256_file(tools.pdfinfo))

    def test_nonzero_conversion_fails_without_publishing_output(self) -> None:
        # Given
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            tools = self._tools(root)

            def fail_conversion(request: LegacyProcessRequest) -> int:
                if request.command[1:] == ("--version",):
                    request.stdout_path.write_text("tool 1\n", encoding="utf-8")
                    return 0
                if request.command[1:] == ("-v",):
                    request.stderr_path.write_text("tool 1\n", encoding="utf-8")
                    return 0
                return 9

            runtime = build_legacy_runtime(tools, runner=fail_conversion)
            source = root / "source.docx"
            write_positive_source(source, "docx", "source")
            destination = root / "result.doc"

            # When / Then
            with self.assertRaisesRegex(
                LegacyConformanceError,
                "LibreOffice conversion failed",
            ):
                runtime.materialize(
                    LegacyPairJob(
                        "doc-conformance-001",
                        DocumentFormat.DOC,
                        source,
                        destination,
                        root / "work",
                    )
                )

            self.assertFalse(destination.exists())

    def _tools(self, root: Path) -> LegacyExternalTools:
        soffice = root / "soffice"
        pdfinfo = root / "pdfinfo"
        font = root / "font.ttf"
        font_bundle = root / "font-bundle.json"
        soffice.write_bytes(b"soffice")
        pdfinfo.write_bytes(b"pdfinfo")
        font.write_bytes(b"font")
        font_bundle.write_text(
            json.dumps(
                {
                    "schema_version": 1,
                    "fonts": [
                        {
                            "path": font.name,
                            "sha256": sha256_file(font),
                        }
                    ],
                }
            ),
            encoding="utf-8",
        )
        return LegacyExternalTools(soffice, pdfinfo, font_bundle)

    def _run(
        self,
        requests: list[LegacyProcessRequest],
        request: LegacyProcessRequest,
    ) -> int:
        requests.append(request)
        if request.command[1:] == ("--version",):
            request.stdout_path.write_text("tool 1\n", encoding="utf-8")
            return 0
        if request.command[1:] == ("-v",):
            request.stderr_path.write_text("tool 1\n", encoding="utf-8")
            return 0
        if request.command[0].endswith("pdfinfo") and len(request.command) == 2:
            request.stdout_path.write_text("Pages: 2\n", encoding="utf-8")
            return 0
        convert_to = request.command[request.command.index("--convert-to") + 1]
        output_dir = Path(request.command[request.command.index("--outdir") + 1])
        source = Path(request.command[-1])
        extension = convert_to.split(":", maxsplit=1)[0]
        output = output_dir / f"{source.stem}.{extension}"
        write_positive_source(output, extension, source.stem)
        return 0

    def _modern(self, document_format: DocumentFormat) -> str:
        return {
            DocumentFormat.DOC: "docx",
            DocumentFormat.XLS: "xlsx",
            DocumentFormat.PPT: "pptx",
        }[document_format]


if __name__ == "__main__":
    unittest.main()
