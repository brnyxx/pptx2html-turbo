from __future__ import annotations

import json
import tempfile
import unittest
import zlib
from dataclasses import replace
from pathlib import Path
from unittest import mock

from evaluate.multiformat_candidate_sources import CandidateSource, CandidateUnitSpec
from evaluate.multiformat_portable_reference_runner import (
    PortableReferenceRunError,
    PortableReferenceTools,
    run_reference_source,
)
from evaluate.multiformat_reference_routing import (
    DocumentFormat,
    load_reference_routing,
)
from evaluate.multiformat_schema import sha256_file

ROUTING = Path(__file__).resolve().parents[1] / "multiformat/reference-routing.v1.json"


class PortableReferenceRunnerTests(unittest.TestCase):
    def test_exact_sandboxed_presentation_argv_and_outputs(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source = root / "input.pptx"
            source.write_bytes(b"presentation")
            tools = self._tools(root, 960, 540)
            runtime_verifier = mock.Mock(wraps=tools.verify_runtime)
            tools = replace(tools, verify_runtime=runtime_verifier)
            spec = CandidateSource(
                "conformance",
                "deck",
                sha256_file(source),
                source,
                (CandidateUnitSpec("slide-1", 1),),
            )
            with mock.patch(
                "evaluate.multiformat_portable_reference_runner.canonicalize_pdf_bytes",
                return_value=b"canonical-pdf",
            ) as canonicalize:
                result = run_reference_source(
                    spec,
                    DocumentFormat.PPTX,
                    load_reference_routing(ROUTING),
                    tools,
                    root / "output",
                )
            render_args = json.loads((root / "output/pdftoppm.argv.json").read_text())
            self.assertEqual(
                render_args[0:5], ["-png", "-scale-to-x", "960", "-scale-to-y", "540"]
            )
            self.assertEqual(
                (result.units[0].width, result.units[0].height), (960, 540)
            )
            canonicalize.assert_called_once()
            self.assertEqual(runtime_verifier.call_count, 4)

    def test_xls_semantic_process_uses_next_route_log_index(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source = root / "input.xls"
            source.write_bytes(b"spreadsheet")
            tools = self._tools(root, 960, 540)
            spec = CandidateSource(
                "conformance",
                "sheet",
                sha256_file(source),
                source,
                (CandidateUnitSpec("sheet-1", 1),),
            )
            routing = load_reference_routing(ROUTING)
            xls_route = next(
                route for route in routing.routes if route.format is DocumentFormat.XLS
            )
            expanded = replace(
                xls_route, commands=(*xls_route.commands, xls_route.commands[-1])
            )
            routing = replace(
                routing,
                routes=tuple(
                    expanded if route is xls_route else route
                    for route in routing.routes
                ),
            )
            with (
                mock.patch(
                    "evaluate.multiformat_portable_reference_runner.canonicalize_pdf_bytes",
                    return_value=b"pdf",
                ),
                mock.patch(
                    "evaluate.multiformat_portable_reference_runner.extract_xlsx_semantics",
                    return_value={},
                ),
                mock.patch(
                    "evaluate.multiformat_portable_reference_runner._convert_xls_semantics",
                    return_value=root / "semantic.xlsx",
                ) as convert,
            ):
                run_reference_source(
                    spec, DocumentFormat.XLS, routing, tools, root / "output"
                )
            self.assertEqual(convert.call_args.args[-1], 6)

    def test_source_runtime_drift_and_dimension_mismatch_fail(self) -> None:
        for attack in ("drift", "runtime", "dimensions"):
            with self.subTest(attack=attack), tempfile.TemporaryDirectory() as temp_dir:
                root = Path(temp_dir)
                source = root / "input.pptx"
                source.write_bytes(b"presentation")
                tools = self._tools(
                    root,
                    10 if attack == "dimensions" else 960,
                    10 if attack == "dimensions" else 540,
                )
                spec = CandidateSource(
                    "conformance",
                    "deck",
                    sha256_file(source),
                    source,
                    (CandidateUnitSpec("slide-1", 1),),
                )

                def mutation(
                    path: Path,
                    attack: str = attack,
                    tools: PortableReferenceTools = tools,
                ) -> None:
                    if attack == "drift":
                        path.write_bytes(b"changed")
                    elif attack == "runtime":
                        tools.poppler_render.write_bytes(b"changed")

                with (
                    mock.patch(
                        "evaluate.multiformat_portable_reference_runner.canonicalize_pdf_bytes",
                        return_value=b"pdf",
                    ),
                    mock.patch(
                        "evaluate.multiformat_portable_reference_runner._after_command",
                        side_effect=mutation,
                    ),
                    self.assertRaises(PortableReferenceRunError),
                ):
                    run_reference_source(
                        spec,
                        DocumentFormat.PPTX,
                        load_reference_routing(ROUTING),
                        tools,
                        root / "output",
                    )

    def _tools(self, root: Path, width: int, height: int) -> PortableReferenceTools:
        profile = root / "sandbox.sb"
        profile.write_text(
            "(version 1)\n"
            "(allow default)\n"
            "(deny network*)\n"
            '(if (param "LIBREOFFICE")\n'
            '  (with-filter (process-path (param "LIBREOFFICE"))\n'
            "    (allow network-bind\n"
            '      (local unix-socket (regex #"^/private/tmp/'
            'OSL_PIPE_[0-9]+_SingleOfficeIPC_[0-9a-f]+$")))))\n'
            '(deny file-read* (subpath (param "ORACLE_ROOT")))\n'
        )
        soffice = self._script(
            root,
            "soffice",
            "import json,pathlib,sys\np=pathlib.Path.cwd();(p/'soffice.argv.json').write_text(json.dumps(sys.argv[1:]));(p/'source.pdf').write_bytes(b'raw')",
        )
        pdfinfo = self._script(
            root,
            "pdfinfo",
            "import json,pathlib,sys\np=pathlib.Path.cwd();(p/'pdfinfo.argv.json').write_text(json.dumps(sys.argv[1:]));print('Pages: 1')",
        )
        png = _png(width, height)
        (root / "fixture.png").write_bytes(png)
        pdftoppm = self._script(
            root,
            "pdftoppm",
            "import json,pathlib,shutil,sys\np=pathlib.Path.cwd();(p/'pdftoppm.argv.json').write_text(json.dumps(sys.argv[1:]));shutil.copyfile(p.parent/'fixture.png',p/'page-1.png')",
        )
        pdftotext = self._script(
            root,
            "pdftotext",
            'import json,pathlib,sys\np=pathlib.Path.cwd();(p/\'pdftotext.argv.json\').write_text(json.dumps(sys.argv[1:]));pathlib.Path(sys.argv[-1]).write_text(\'<doc><page width="960" height="540"><line><word xMin="1" yMin="1" xMax="2" yMax="2">x</word></line></page></doc>\')',
        )
        runtime = (soffice, pdfinfo, pdftoppm, pdftotext, profile)
        expected = {path: sha256_file(path) for path in runtime}

        def verify_runtime() -> None:
            if any(sha256_file(path) != digest for path, digest in expected.items()):
                raise PortableReferenceRunError("portable reference runtime drifted")

        return PortableReferenceTools(
            soffice,
            pdfinfo,
            pdftoppm,
            pdftotext,
            Path("/usr/bin/sandbox-exec"),
            profile,
            verify_runtime,
        )

    @staticmethod
    def _script(root: Path, name: str, body: str) -> Path:
        path = root / name
        path.write_text("#!/usr/bin/env python3\n" + body + "\n")
        path.chmod(0o755)
        return path


def _png(width: int, height: int) -> bytes:
    def chunk(kind: bytes, value: bytes) -> bytes:
        return (
            len(value).to_bytes(4, "big")
            + kind
            + value
            + (zlib.crc32(kind + value) & 0xFFFFFFFF).to_bytes(4, "big")
        )

    rows = b"".join(b"\0" + b"\xff\xff\xff" * width for _ in range(height))
    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(
            b"IHDR",
            width.to_bytes(4, "big") + height.to_bytes(4, "big") + b"\x08\x02\0\0\0",
        )
        + chunk(b"IDAT", zlib.compress(rows))
        + chunk(b"IEND", b"")
    )


if __name__ == "__main__":
    unittest.main()
