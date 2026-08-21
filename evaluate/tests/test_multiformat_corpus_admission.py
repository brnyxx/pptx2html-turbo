from __future__ import annotations

import json
import struct
import tempfile
import unittest
from decimal import Decimal
from pathlib import Path
from unittest import mock

from evaluate.multiformat_corpus_admission import (
    AdmissionMetadata,
    AdmissionPlan,
    AdmissionSource,
    AdmissionValidators,
    admit_corpus,
)
from evaluate.multiformat_corpus_identity import validate_admitted_corpus
from evaluate.multiformat_corpus_types import CorpusError, CorpusStatus
from evaluate.multiformat_gate import GateStatus, evaluate_reports
from evaluate.multiformat_inventory_types import Box
from evaluate.multiformat_metric_compute import UnitArtifacts
from evaluate.multiformat_metric_types import VisualScores
from evaluate.multiformat_schema import (
    JsonValue,
    object_value,
    sha256_file,
    string_value,
)
from evaluate.tests.multiformat_corpus_fixture import write_corpus
from evaluate.tests.multiformat_gate_fixture import MultiFormatGateFixture

PROJECT_ROOT = Path(__file__).resolve().parents[2]
CONTRACT = PROJECT_ROOT / "evaluate" / "multiformat" / "contract.v1.json"


class MultiFormatCorpusAdmissionTests(unittest.TestCase):
    def test_injected_qualification_admits_all_formats_atomically(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            manifests = self._write_input_corpora(root)
            destination = root / "admitted"
            calls: list[tuple[str, str]] = []

            def qualify(source: AdmissionSource) -> bytes:
                calls.append((source.document_format.value, source.item_id))
                return f"qualified:{source.document_format.value}:{source.item_id}".encode()

            result = admit_corpus(
                self._plan(manifests, destination),
                AdmissionValidators(
                    extraction=qualify,
                    fonts=qualify,
                    rendering=qualify,
                ),
            )

            self.assertEqual(result.status, CorpusStatus.READY)
            self.assertTrue((destination / "READY").is_file())
            self.assertEqual(len(calls), 608 * 3)
            validation = validate_admitted_corpus(CONTRACT, destination)
            self.assertEqual(validation.aggregate_sha256, result.aggregate_sha256)

    def test_missing_real_qualification_stays_incomplete_without_publication(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            manifests = self._write_input_corpora(root)
            destination = root / "admitted"

            result = admit_corpus(self._plan(manifests, destination), None)

            self.assertEqual(result.status, CorpusStatus.INCOMPLETE)
            self.assertFalse(destination.exists())
            self.assertIn("qualification", result.reasons)

    def test_validator_consumes_staged_copy_when_original_changes(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            manifests = self._write_input_corpora(root)
            destination = root / "admitted"
            first_manifest = json.loads(manifests[0].read_text(encoding="utf-8"))
            first_item = first_manifest["tracks"]["conformance"]["items"][0]
            original = manifests[0].parent / first_item["path"]
            original_bytes = original.read_bytes()
            changed = False

            def mutate_original(source: AdmissionSource) -> bytes:
                nonlocal changed
                if not changed:
                    changed = True
                    original.write_bytes(b"mutated original")
                if source.path.resolve() == original.resolve():
                    raise CorpusError("admission.isolation", source.item_id)
                return source.item_id.encode()

            result = admit_corpus(
                self._plan(manifests, destination),
                AdmissionValidators(
                    extraction=mutate_original,
                    fonts=lambda source: source.item_id.encode(),
                    rendering=lambda source: source.item_id.encode(),
                ),
            )

            admitted = destination / "sources" / "pptx" / original.name
            admitted_manifest = json.loads(
                (destination / "manifest.json").read_text(encoding="utf-8")
            )
            admitted_record = next(
                item
                for item in admitted_manifest["sources"]
                if item["format"] == "pptx" and item["id"] == first_item["id"]
            )
            extraction = destination / admitted_record["evidence"]["extraction"]["path"]
            self.assertEqual(result.status, CorpusStatus.READY)
            self.assertEqual(admitted.read_bytes(), original_bytes)
            self.assertEqual(extraction.read_bytes(), first_item["id"].encode())
            self.assertEqual(original.read_bytes(), b"mutated original")

    def test_staged_copy_mutation_fails_without_publication(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            manifests = self._write_input_corpora(root)
            destination = root / "admitted"
            changed = False

            def mutate_staged(source: AdmissionSource) -> bytes:
                nonlocal changed
                if not changed:
                    changed = True
                    source.path.write_bytes(b"mutated staged copy")
                return source.item_id.encode()

            result = admit_corpus(
                self._plan(manifests, destination),
                AdmissionValidators(
                    extraction=mutate_staged,
                    fonts=lambda source: source.item_id.encode(),
                    rendering=lambda source: source.item_id.encode(),
                ),
            )

            self.assertEqual(result.status, CorpusStatus.INCOMPLETE)
            self.assertEqual(result.reasons, ("corpus.sources_changed",))
            self.assertFalse(destination.exists())
            self.assertEqual(tuple(root.glob(".admitted.stage-*")), ())

    def test_validator_failure_leaves_no_ready_or_partial_tree(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            manifests = self._write_input_corpora(root)
            destination = root / "admitted"

            def fail_validation(source: AdmissionSource) -> bytes:
                raise CorpusError("admission.render", source.item_id)

            result = admit_corpus(
                self._plan(manifests, destination),
                AdmissionValidators(
                    extraction=lambda source: source.item_id.encode(),
                    fonts=lambda source: source.item_id.encode(),
                    rendering=fail_validation,
                ),
            )

            self.assertEqual(result.status, CorpusStatus.INCOMPLETE)
            self.assertFalse(destination.exists())
            self.assertEqual(tuple(root.glob(".admitted.stage-*")), ())

    def test_admission_and_validation_never_call_corpus_generators(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            manifests = self._write_input_corpora(root)
            destination = root / "admitted"
            validators = AdmissionValidators(
                extraction=lambda source: source.item_id.encode(),
                fonts=lambda source: source.item_id.encode(),
                rendering=lambda source: source.item_id.encode(),
            )
            targets = (
                "evaluate.generate_multiformat_docx_conformance.generate_docx_conformance",
                "evaluate.generate_multiformat_xlsx_conformance.generate_xlsx_conformance",
                "evaluate.generate_multiformat_pptx_conformance.generate_pptx_conformance",
                "evaluate.generate_multiformat_pdf_conformance.generate_pdf_conformance",
            )
            patches = [
                mock.patch(target, side_effect=AssertionError(target))
                for target in targets
            ]
            for patcher in patches:
                patcher.start()
            self.addCleanup(lambda: [patcher.stop() for patcher in reversed(patches)])

            result = admit_corpus(
                self._plan(manifests, destination),
                validators,
            )
            validation = validate_admitted_corpus(CONTRACT, destination)
            reports = root / "gate" / "reports"
            reports.mkdir(parents=True)
            gate_fixture = MultiFormatGateFixture()
            lock = gate_fixture._write_oracle_lock(root / "gate")
            gate_fixture._write_reports(reports, lock)
            evaluator_lock = json.loads(
                (CONTRACT.parent / "evaluator-lock.v1.json").read_text(encoding="utf-8")
            )
            dependencies = object_value(evaluator_lock, "dependencies")

            def fixture_visual(
                artifacts: UnitArtifacts,
                _background: str,
                _boxes: tuple[Box, ...],
            ) -> tuple[VisualScores, tuple[int, int]]:
                data = artifacts.reference_png.read_bytes()
                dimensions = struct.unpack(">II", data[16:24])
                score = Decimal(100)
                return VisualScores(score, score, score, score), dimensions

            with (
                mock.patch(
                    "evaluate.multiformat_evaluator_manifest.importlib.metadata.version",
                    side_effect=lambda name: string_value(dependencies, name),
                ),
                mock.patch(
                    "evaluate.multiformat_metric_compute._cached_visual",
                    side_effect=fixture_visual,
                ),
            ):
                summary = evaluate_reports(CONTRACT, reports, lock)

            self.assertEqual(result.status, CorpusStatus.READY)
            self.assertEqual(validation.status, CorpusStatus.READY)
            self.assertEqual(
                summary.status,
                GateStatus.PASS,
                [(item.format, item.reasons) for item in summary.formats],
            )

    def _plan(self, manifests: tuple[Path, ...], destination: Path) -> AdmissionPlan:
        return AdmissionPlan(
            contract_path=CONTRACT,
            corpus_manifests=manifests,
            destination=destination,
            metadata=AdmissionMetadata(
                corpus_revision="fixture-corpus-v1",
                project_revision="b" * 40,
                admitted_at="2026-08-21T00:00:00Z",
            ),
        )

    def _write_input_corpora(self, root: Path) -> tuple[Path, ...]:
        contract = json.loads(CONTRACT.read_text(encoding="utf-8"))
        strata = object_value(contract, "stratum_quotas")
        outcomes = object_value(contract, "security_case_outcomes")
        paired = object_value(contract, "legacy_paired_stratum_quotas")
        manifests: list[Path] = []
        for document_format in ("pptx", "docx", "doc", "xlsx", "xls", "ppt", "pdf"):
            quota_values = object_value(strata, document_format)
            security_values = object_value(outcomes, document_format)
            paired_values: dict[str, JsonValue] | None = None
            if document_format in {"doc", "xls", "ppt"}:
                paired_values = object_value(paired, document_format)
            manifests.append(
                write_corpus(
                    root / "input",
                    document_format,
                    sha256_file(CONTRACT),
                    quota_values,
                    security_values,
                    paired_values,
                )
            )
        return tuple(manifests)


def write_input_corpora(root: Path) -> tuple[Path, ...]:
    return MultiFormatCorpusAdmissionTests()._write_input_corpora(root)


if __name__ == "__main__":
    unittest.main()
