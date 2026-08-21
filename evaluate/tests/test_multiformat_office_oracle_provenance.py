from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from evaluate.multiformat_metric_types import MetricError
from evaluate.multiformat_office_oracle_batch import (
    OfficeOracleBatch,
    OfficeOracleBatchFile,
    OfficeOracleBatchUnit,
)
from evaluate.multiformat_office_oracle_provenance import (
    _validate_batch_units,
)
from evaluate.multiformat_schema import sha256_file


class MultiFormatOfficeOracleProvenanceTests(unittest.TestCase):
    def test_batch_sources_must_exactly_match_capture_units(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            batch = self._batch(root)

            with self.assertRaisesRegex(MetricError, "source set"):
                _validate_batch_units(batch, [], root)

    def test_batch_png_must_match_capture_unit(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            batch = self._batch(root)
            other = root / "other.png"
            other.write_bytes(b"other")
            units = [
                {
                    "unit_id": "unit-1",
                    "source_id": "source-1",
                    "source_sha256": "1" * 64,
                    "ordinal": 1,
                    "png": {
                        "path": other.name,
                        "sha256": sha256_file(other),
                    },
                    "inventory": {
                        "path": other.name,
                        "sha256": sha256_file(other),
                    },
                }
            ]

            with self.assertRaisesRegex(MetricError, "png set"):
                _validate_batch_units(batch, units, root)

    def _batch(self, root: Path) -> OfficeOracleBatch:
        png = root / "unit.png"
        pdf = root / "reference.pdf"
        semantic = root / "semantic.json"
        layout = root / "layout.xml"
        manifest = root / "manifest.json"
        png.write_bytes(b"png")
        pdf.write_bytes(b"pdf")
        semantic.write_text("{}", encoding="utf-8")
        layout.write_text("<doc/>", encoding="utf-8")
        manifest.write_text("{}", encoding="utf-8")
        batch_file = OfficeOracleBatchFile(
            "source-1",
            "docx",
            "1" * 64,
            pdf,
            semantic,
            layout,
            (OfficeOracleBatchUnit(png, 1, 1),),
        )
        return OfficeOracleBatch(
            manifest,
            "batch-1",
            "2026-08-21T00:00:00Z",
            "2" * 40,
            "3" * 64,
            {},
            {"source-1": batch_file},
            frozenset({png, pdf, semantic, layout}),
        )
