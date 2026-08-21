from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from evaluate.build_multiformat_office_input import (
    OfficeInputBuildError,
    build_office_input_bundle,
)
from evaluate.multiformat_schema import sha256_file
from evaluate.tests.multiformat_small_corpus_fixture import ready_fixture


class BuildMultiFormatOfficeInputTests(unittest.TestCase):
    def test_bundle_copies_exact_positive_sources_and_gate_inputs(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture_root = root / "fixture"
            fixture_root.mkdir()
            contract, corpus = ready_fixture(fixture_root)
            evaluator = root / "evaluator.json"
            lock = root / "oracle-lock.json"
            evaluator.write_text('{"schema_version":1}', encoding="utf-8")
            lock.write_text('{"schema_version":1}', encoding="utf-8")

            output = build_office_input_bundle(
                contract=contract,
                corpus_manifests=[corpus],
                evaluator_manifest=evaluator,
                oracle_lock=lock,
                output_dir=root / "bundle",
            )

            values = json.loads(output.read_text(encoding="utf-8"))
            self.assertEqual(values["schema_version"], 1)
            self.assertEqual(len(values["files"]), 6)
            for item in values["files"]:
                source = output.parent / item["path"]
                self.assertTrue(source.is_file())
                self.assertEqual(sha256_file(source), item["sha256"])

    def test_missing_required_format_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture_root = root / "fixture"
            fixture_root.mkdir()
            contract, _ = ready_fixture(fixture_root)
            evaluator = root / "evaluator.json"
            lock = root / "oracle-lock.json"
            evaluator.write_text("{}", encoding="utf-8")
            lock.write_text("{}", encoding="utf-8")

            with self.assertRaisesRegex(
                OfficeInputBuildError,
                "required format",
            ):
                build_office_input_bundle(
                    contract=contract,
                    corpus_manifests=[],
                    evaluator_manifest=evaluator,
                    oracle_lock=lock,
                    output_dir=root / "bundle",
                )

    def test_corpus_schema_errors_are_wrapped(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture_root = root / "fixture"
            fixture_root.mkdir()
            contract, corpus = ready_fixture(fixture_root)
            values = json.loads(corpus.read_text(encoding="utf-8"))
            values["tracks"]["conformance"]["items"] = {}
            corpus.write_text(json.dumps(values), encoding="utf-8")
            evaluator = root / "evaluator.json"
            lock = root / "oracle-lock.json"
            evaluator.write_text("{}", encoding="utf-8")
            lock.write_text("{}", encoding="utf-8")

            with self.assertRaises(OfficeInputBuildError):
                build_office_input_bundle(
                    contract=contract,
                    corpus_manifests=[corpus],
                    evaluator_manifest=evaluator,
                    oracle_lock=lock,
                    output_dir=root / "bundle",
                )
