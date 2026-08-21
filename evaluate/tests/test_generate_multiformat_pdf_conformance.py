from __future__ import annotations

import json
import shutil
import tempfile
import unittest
from pathlib import Path

from evaluate.build_multiformat_conformance_plan import build_conformance_plan
from evaluate.generate_multiformat_pdf_conformance import (
    generate_pdf_conformance,
)
from evaluate.multiformat_schema import sha256_file
from evaluate.tests.multiformat_source_fixture import write_positive_source

PROJECT_ROOT = Path(__file__).resolve().parents[2]
CONTRACT = PROJECT_ROOT / "evaluate" / "multiformat" / "contract.v1.json"


class GenerateMultiFormatPdfConformanceTests(unittest.TestCase):
    def test_generates_exact_100_source_manifest(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            plan = root / "plan.json"
            build_conformance_plan(CONTRACT, plan)

            manifest = generate_pdf_conformance(
                CONTRACT,
                plan,
                root / "output",
                converter=self._convert,
                canonicalizer=lambda source, destination: shutil.copy2(
                    source,
                    destination,
                ),
                page_counter=lambda path: 1,
                tools={
                    "soffice_sha256": "1" * 64,
                    "soffice_version": "test-soffice",
                    "pdfinfo_sha256": "2" * 64,
                    "pdfinfo_version": "test-pdfinfo",
                    "font_environment_sha256": "3" * 64,
                    "pdftocairo_sha256": "4" * 64,
                    "pdftocairo_version": "test-pdftocairo",
                },
            )

            values = json.loads(manifest.read_text(encoding="utf-8"))
            self.assertEqual(len(values["files"]), 100)
            self.assertTrue(all(item["unit_count"] == 1 for item in values["files"]))
            self.assertTrue(
                all(
                    sha256_file(manifest.parent / item["path"]) == item["sha256"]
                    for item in values["files"]
                )
            )
            self.assertEqual(
                len([path for path in manifest.parent.rglob("*") if path.is_file()]),
                101,
            )

    def _convert(
        self,
        html_paths: tuple[Path, ...],
        output_dir: Path,
        profile_dir: Path,
    ) -> None:
        for html_path in html_paths:
            write_positive_source(
                output_dir / f"{html_path.stem}.pdf",
                "pdf",
                html_path.stem,
            )
