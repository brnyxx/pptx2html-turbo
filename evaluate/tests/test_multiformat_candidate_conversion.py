import hashlib
import json
import stat
import sys
import tempfile
import time
import unittest
import zipfile
from pathlib import Path

from evaluate.multiformat_candidate_conversion import (
    CandidateConversionError,
    _validate_diagnostics,
    run_conversion,
)
from evaluate.multiformat_corpus_types import DocumentFormat


class MultiFormatCandidateConversionTests(unittest.TestCase):
    def test_truncated_spreadsheet_scan_cannot_back_candidate_evidence(self) -> None:
        with self.assertRaisesRegex(CandidateConversionError, "scan truncated"):
            _validate_diagnostics(
                [
                    {"code": "NATIVE_BACKEND_OPAQUE"},
                    {"code": "SPREADSHEET_CELL_SCAN_TRUNCATED"},
                ],
                DocumentFormat.XLSX,
            )

    def test_invokes_exact_converter_with_isolated_explicit_backends(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source = root / "source.docx"
            source.write_bytes(b"source")
            converter = self._converter(root, exit_code=0)
            soffice = self._tool(root, "soffice")
            pdftohtml = self._tool(root, "pdftohtml")
            pdfinfo = self._tool(root, "pdfinfo")

            result = run_conversion(
                converter,
                source,
                DocumentFormat.DOCX,
                root / "run",
                soffice=soffice,
                pdftohtml=pdftohtml,
                pdfinfo=pdfinfo,
                timeout_seconds=30,
            )

            self.assertIn("page1-div", result.html)
            diagnostics = json.loads(result.diagnostics.read_text(encoding="utf-8"))
            args = diagnostics[0]["args"]
            self.assertIn("--input-format", args)
            self.assertIn("docx", args)
            self.assertIn("--soffice", args)
            self.assertIn(soffice.resolve().as_posix(), args)
            self.assertNotIn("--allow-unisolated", args)
            self.assertEqual(result.source_sha256, self._sha256(source))

    def test_nonzero_converter_exit_fails_without_publishing_html(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source = root / "source.pdf"
            source.write_bytes(b"source")
            converter = self._converter(root, exit_code=7)
            tool = self._tool(root, "tool")

            with self.assertRaisesRegex(
                CandidateConversionError,
                "exit code 7",
            ):
                run_conversion(
                    converter,
                    source,
                    DocumentFormat.PDF,
                    root / "run",
                    soffice=tool,
                    pdftohtml=tool,
                    pdfinfo=tool,
                    timeout_seconds=30,
                )

    def test_pptx_capture_uses_uniform_scale_to_exact_960_by_540(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source = root / "source.pptx"
            self._pptx(source, 12_192_000, 6_858_000)
            converter = self._converter(root, exit_code=0)
            tool = self._tool(root, "tool")

            result = run_conversion(
                converter,
                source,
                DocumentFormat.PPTX,
                root / "run",
                soffice=tool,
                pdftohtml=tool,
                pdfinfo=tool,
                timeout_seconds=30,
            )

            diagnostics = json.loads(result.diagnostics.read_text(encoding="utf-8"))
            args = diagnostics[0]["args"]
            scale = float(args[args.index("--presentation-scale") + 1])
            self.assertAlmostEqual(scale, 0.75, places=6)

    def test_pptx_capture_scales_four_by_three_slide_to_canonical_width(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source = root / "source.pptx"
            self._pptx(source, 9_144_000, 6_858_000)
            converter = self._converter(root, exit_code=0)
            tool = self._tool(root, "tool")

            result = run_conversion(
                converter,
                source,
                DocumentFormat.PPTX,
                root / "run",
                soffice=tool,
                pdftohtml=tool,
                pdfinfo=tool,
                timeout_seconds=30,
            )

            diagnostics = json.loads(result.diagnostics.read_text(encoding="utf-8"))
            args = diagnostics[0]["args"]
            scale = float(args[args.index("--presentation-scale") + 1])
            self.assertAlmostEqual(scale, 1.0, places=6)

    def test_kills_converter_when_a_log_crosses_the_bound(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source = root / "source.pdf"
            source.write_bytes(b"source")
            converter = root / "noisy.py"
            converter.write_text(
                f"#!{sys.executable}\n"
                "import sys\n"
                "sys.stdout.write('x' * (8 * 1024 * 1024 + 1))\n",
                encoding="utf-8",
            )
            converter.chmod(converter.stat().st_mode | stat.S_IXUSR)
            tool = self._tool(root, "tool")

            with self.assertRaisesRegex(
                CandidateConversionError,
                "log exceeds",
            ):
                run_conversion(
                    converter,
                    source,
                    DocumentFormat.PDF,
                    root / "run",
                    soffice=tool,
                    pdftohtml=tool,
                    pdfinfo=tool,
                    timeout_seconds=30,
                )

    def test_descendant_cannot_hold_capture_pipes_past_deadline(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source = root / "source.pdf"
            source.write_bytes(b"source")
            converter = root / "descendant.py"
            converter.write_text(
                "#!/bin/sh\nsleep 2 &\nexit 0\n",
                encoding="utf-8",
            )
            converter.chmod(converter.stat().st_mode | stat.S_IXUSR)
            tool = self._tool(root, "tool")

            started = time.monotonic()
            with self.assertRaises(CandidateConversionError):
                run_conversion(
                    converter,
                    source,
                    DocumentFormat.PDF,
                    root / "run",
                    soffice=tool,
                    pdftohtml=tool,
                    pdfinfo=tool,
                    timeout_seconds=0.2,
                )
            self.assertLess(time.monotonic() - started, 1.0)

    @classmethod
    def _converter(cls, root: Path, *, exit_code: int) -> Path:
        path = root / f"converter-{exit_code}.py"
        path.write_text(
            f"""#!{sys.executable}
import json
import pathlib
import sys
args = sys.argv[1:]
if {exit_code}:
    raise SystemExit({exit_code})
output = pathlib.Path(args[args.index("--output") + 1])
diagnostics = pathlib.Path(args[args.index("--diagnostics") + 1])
output.write_text('<html><body><div id="page1-div" style="width:100px;height:100px"></div></body></html>')
diagnostics.write_text(json.dumps([{{"code": "NATIVE_BACKEND_OPAQUE", "args": args}}]))
""",
            encoding="utf-8",
        )
        path.chmod(path.stat().st_mode | stat.S_IXUSR)
        return path

    @staticmethod
    def _tool(root: Path, name: str) -> Path:
        path = root / name
        path.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
        path.chmod(path.stat().st_mode | stat.S_IXUSR)
        return path

    @staticmethod
    def _pptx(path: Path, width: int, height: int) -> None:
        with zipfile.ZipFile(path, "w") as archive:
            archive.writestr(
                "ppt/presentation.xml",
                (
                    '<p:presentation xmlns:p="http://schemas.openxmlformats.org/'
                    'presentationml/2006/main"><p:sldSz '
                    f'cx="{width}" cy="{height}"/></p:presentation>'
                ),
            )

    @staticmethod
    def _sha256(path: Path) -> str:
        return hashlib.sha256(path.read_bytes()).hexdigest()
