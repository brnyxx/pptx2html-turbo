from __future__ import annotations

import tempfile
import unittest
from pathlib import Path
from unittest import mock

from evaluate.generate_multiformat_legacy_conformance import main
from evaluate.multiformat_legacy_types import LegacyConformanceError


class GenerateMultiFormatLegacyConformanceCliTests(unittest.TestCase):
    def test_cli_binds_all_inputs_and_locked_tools(self) -> None:
        # Given
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            arguments = self._arguments(root)
            runtime = object()
            with (
                mock.patch(
                    "evaluate.generate_multiformat_legacy_conformance."
                    "build_legacy_runtime",
                    return_value=runtime,
                ) as build_runtime,
                mock.patch(
                    "evaluate.generate_multiformat_legacy_conformance."
                    "generate_legacy_pairs",
                ) as generate,
            ):
                # When
                exit_code = main(arguments)

            # Then
            self.assertEqual(exit_code, 0)
            tools = build_runtime.call_args.args[0]
            self.assertEqual(tools.soffice, root / "soffice")
            self.assertEqual(tools.pdfinfo, root / "pdfinfo")
            self.assertEqual(tools.font_bundle, root / "font-bundle.json")
            request, bound_runtime = generate.call_args.args
            self.assertIs(bound_runtime, runtime)
            self.assertEqual(request.contract, root / "contract.json")
            self.assertEqual(request.plan, root / "plan.json")
            self.assertEqual(
                request.modern_manifests,
                (
                    root / "docx.json",
                    root / "xlsx.json",
                    root / "pptx.json",
                ),
            )
            self.assertEqual(request.output_dir, root / "output")

    def test_cli_reports_typed_generation_failure(self) -> None:
        # Given
        with tempfile.TemporaryDirectory() as temp_dir:
            arguments = self._arguments(Path(temp_dir))
            with (
                mock.patch(
                    "evaluate.generate_multiformat_legacy_conformance."
                    "build_legacy_runtime",
                    side_effect=LegacyConformanceError("locked tool differs"),
                ),
                self.assertRaises(SystemExit) as raised,
            ):
                # When
                main(arguments)

            # Then
            self.assertEqual(raised.exception.code, 2)

    def _arguments(self, root: Path) -> list[str]:
        return [
            "--contract",
            str(root / "contract.json"),
            "--plan",
            str(root / "plan.json"),
            "--docx-manifest",
            str(root / "docx.json"),
            "--xlsx-manifest",
            str(root / "xlsx.json"),
            "--pptx-manifest",
            str(root / "pptx.json"),
            "--output-dir",
            str(root / "output"),
            "--soffice",
            str(root / "soffice"),
            "--pdfinfo",
            str(root / "pdfinfo"),
            "--font-bundle",
            str(root / "font-bundle.json"),
        ]


if __name__ == "__main__":
    unittest.main()
