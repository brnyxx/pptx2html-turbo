from __future__ import annotations

import tempfile
import unittest
from dataclasses import replace
from pathlib import Path

from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_native_unit_manifest import (
    NativeManifestInputs,
    build_native_unit_manifest,
)
from evaluate.multiformat_native_unit_runtime import capture_native_observation
from evaluate.multiformat_native_unit_types import NativeUnitError
from evaluate.tests.multiformat_native_unit_fixture import (
    RecordingNativeRunner,
    make_native_unit_fixture,
)


class MultiFormatNativeUnitErrorScopeTests(unittest.TestCase):
    def test_pair_disagreement_retains_source_scope(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_unit_fixture(root)
            runner = RecordingNativeRunner()
            first = capture_native_observation(
                fixture.request(root / "run-1", DocumentFormat.DOCX, run=1),
                runner,
            )
            second = capture_native_observation(
                fixture.request(root / "run-2", DocumentFormat.DOCX, run=2),
                runner,
            )

            with self.assertRaises(NativeUnitError) as raised:
                _ = build_native_unit_manifest(
                    NativeManifestInputs(
                        root / "contract",
                        root / "config",
                        root / "manifest",
                        1,
                        "macos",
                        "arm64",
                        "a" * 64,
                        "b" * 64,
                    ),
                    (first, replace(second, unit_count=second.unit_count + 1)),
                )

        self.assertIs(raised.exception.document_format, DocumentFormat.DOCX)
        self.assertEqual(raised.exception.source_id, "blind-docx-001")


if __name__ == "__main__":
    _ = unittest.main()
