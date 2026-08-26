from __future__ import annotations

import tempfile
import unittest
from dataclasses import replace
from pathlib import Path

from evaluate.jcs import canonicalize
from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_native_unit_manifest import (
    NativeManifestInputs,
    build_native_unit_manifest,
)
from evaluate.multiformat_native_unit_runtime import capture_native_observation
from evaluate.multiformat_native_unit_types import NativeObservation, NativeUnitError
from evaluate.multiformat_schema import object_value
from evaluate.multiformat_strict_json import read_strict_object
from evaluate.tests.multiformat_native_unit_fixture import (
    RecordingNativeRunner,
    make_native_unit_fixture,
)


class MultiFormatNativeUnitErrorScopeTests(unittest.TestCase):
    def test_pair_disagreement_retains_source_scope(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            first, second = self._observations(root)

            with self.assertRaises(NativeUnitError) as raised:
                _ = build_native_unit_manifest(
                    self._inputs(root),
                    (first, replace(second, unit_count=second.unit_count + 1)),
                )

        self._assert_source_scope(raised.exception)

    def test_malformed_execution_record_retains_source_scope(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            observations = self._observations(root)
            _ = observations[0].execution_path.write_bytes(b"{")

            with self.assertRaises(NativeUnitError) as raised:
                _ = build_native_unit_manifest(self._inputs(root), observations)

        self._assert_source_scope(raised.exception)

    def test_invalid_tool_identity_retains_source_scope(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            observations = self._observations(root)
            execution = read_strict_object(observations[0].execution_path)
            tools = object_value(execution, "tools")
            tools["pdfinfo"] = []
            _ = observations[0].execution_path.write_bytes(
                canonicalize(execution) + b"\n"
            )

            with self.assertRaises(NativeUnitError) as raised:
                _ = build_native_unit_manifest(self._inputs(root), observations)

        self._assert_source_scope(raised.exception)

    @staticmethod
    def _observations(root: Path) -> tuple[NativeObservation, NativeObservation]:
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
        return first, second

    @staticmethod
    def _inputs(root: Path) -> NativeManifestInputs:
        return NativeManifestInputs(
            root / "contract",
            root / "config",
            root / "manifest",
            1,
            "macos",
            "arm64",
            "a" * 64,
            "b" * 64,
        )

    def _assert_source_scope(self, error: NativeUnitError) -> None:
        self.assertIs(error.document_format, DocumentFormat.DOCX)
        self.assertEqual(error.source_id, "blind-docx-001")


if __name__ == "__main__":
    _ = unittest.main()
