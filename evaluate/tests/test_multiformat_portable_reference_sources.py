from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from evaluate.multiformat_portable_reference_sources import load_reference_sources
from evaluate.tests.multiformat_small_corpus_fixture import ready_fixture


class PortableReferenceSourceTests(unittest.TestCase):
    def test_ready_manifest_returns_exact_positive_units(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            contract, manifest = ready_fixture(Path(temp_dir))
            sources = load_reference_sources(contract, manifest)
            self.assertEqual(sources.document_format.value, "docx")
            self.assertEqual(sources.sources[0].units[0].unit_id, "unit-1")
            self.assertEqual(sources.sources[0].units[1].ordinal, 2)
            self.assertNotIn("security", {source.track for source in sources.sources})

    def test_source_drift_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            contract, manifest = ready_fixture(Path(temp_dir))
            (manifest.parent / "sources/conformance.docx").write_bytes(b"changed")
            with self.assertRaisesRegex(ValueError, "source digest"):
                load_reference_sources(contract, manifest)


if __name__ == "__main__":
    unittest.main()
