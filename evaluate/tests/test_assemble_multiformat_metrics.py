from __future__ import annotations

import json
import sys
import tempfile
import unittest
from dataclasses import dataclass
from pathlib import Path

from evaluate.assemble_multiformat_metrics import assemble_metrics
from evaluate.assemble_multiformat_report import assemble_report
from evaluate.multiformat_command_evidence import CommandEvidenceError
from evaluate.multiformat_corpus_items import object_list
from evaluate.multiformat_evaluator_files import EVALUATOR_FILES
from evaluate.multiformat_metric_manifest import MetricsAssemblyError
from evaluate.multiformat_metrics import validate_metrics_evidence
from evaluate.multiformat_revision import current_project_revision
from evaluate.multiformat_schema import (
    JsonValue,
    object_value,
    read_object,
    sha256_file,
    string_value,
)
from evaluate.tests.multiformat_candidate_gate_lock_fixture import (
    write_gate_oracle_lock,
)
from evaluate.tests.multiformat_metrics_fixture import write_metrics
from evaluate.tests.multiformat_small_corpus_fixture import ready_fixture

PROJECT_ROOT = Path(__file__).resolve().parents[2]
BASE_CONTRACT = PROJECT_ROOT / "evaluate/multiformat/contract.v1.json"


@dataclass(frozen=True, slots=True)
class Fixture:
    root: Path
    contract: Path
    corpus: Path
    evaluator: Path
    lock: Path
    oracle_capture: Path
    candidate_capture: Path
    reviews: tuple[Path, Path]
    commands: Path


class AssembleMultiformatMetricsTests(unittest.TestCase):
    def test_real_commands_publish_self_validating_metrics_and_report(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            fixture = self._fixture(Path(temporary))
            output = self._assemble(fixture)

            summary = validate_metrics_evidence(
                fixture.contract,
                fixture.corpus,
                output,
                sha256_file(fixture.evaluator),
                sha256_file(fixture.lock),
                fixture.root,
                fixture.lock,
            )
            report = assemble_report(
                PROJECT_ROOT,
                fixture.contract,
                fixture.lock,
                fixture.evaluator,
                fixture.corpus,
                output,
                fixture.root,
            )

            self.assertEqual(summary.conformance.score, 100)
            self.assertEqual(string_value(report, "status"), "READY")
            self.assertEqual(object_value(report, "security")["passed"], 2)
            self.assertTrue(object_value(report, "performance")["within_limits"])

    def test_tampered_capture_artifact_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            fixture = self._fixture(Path(temporary))
            output = self._assemble(fixture)
            candidate = read_object(fixture.candidate_capture)
            unit = object_list(candidate, "units", "capture.units")[0]
            png = fixture.root / string_value(object_value(unit, "png"), "path")
            png.write_bytes(b"tampered")

            with self.assertRaises(MetricsAssemblyError):
                self._assemble(fixture, output=fixture.root / "second.json")
            with self.assertRaises(Exception):
                validate_metrics_evidence(
                    fixture.contract,
                    fixture.corpus,
                    output,
                    sha256_file(fixture.evaluator),
                    sha256_file(fixture.lock),
                    fixture.root,
                    fixture.lock,
                )

    def test_timeout_does_not_publish(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            fixture = self._fixture(Path(temporary), performance_mode="block")
            output = fixture.root / "metrics-produced.json"

            with self.assertRaises(CommandEvidenceError):
                self._assemble(fixture, output=output, timeout=1)
            self.assertFalse(output.exists())

    def test_duplicate_reviewer_identity_fails(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            fixture = self._fixture(Path(temporary))
            second = read_object(fixture.reviews[1])
            second["reviewer_id"] = "reviewer-1"
            fixture.reviews[1].write_text(json.dumps(second), encoding="utf-8")

            with self.assertRaises(MetricsAssemblyError):
                self._assemble(fixture)

    def _assemble(
        self,
        fixture: Fixture,
        *,
        output: Path | None = None,
        timeout: int = 10,
    ) -> Path:
        target = output or fixture.root / "metrics-produced.json"
        assemble_metrics(
            project_root=PROJECT_ROOT,
            contract_path=fixture.contract,
            corpus_path=fixture.corpus,
            evaluator_path=fixture.evaluator,
            oracle_lock_path=fixture.lock,
            oracle_capture_path=fixture.oracle_capture,
            candidate_capture_path=fixture.candidate_capture,
            evidence_root=fixture.root,
            commands_path=fixture.commands,
            review_paths=fixture.reviews,
            execution_output_dir=fixture.root / f"executions-{target.stem}",
            output_path=target,
            timeout_seconds=timeout,
        )
        return target

    def _fixture(self, root: Path, *, performance_mode: str = "pass") -> Fixture:
        contract, corpus = ready_fixture(root)
        contract_value = read_object(contract)
        contract_value["metric_parameters"] = object_value(
            read_object(BASE_CONTRACT), "metric_parameters"
        )
        contract.write_text(
            json.dumps(contract_value, sort_keys=True), encoding="utf-8"
        )
        corpus_value = read_object(corpus)
        corpus_value["contract_sha256"] = sha256_file(contract)
        corpus.write_text(json.dumps(corpus_value, sort_keys=True), encoding="utf-8")
        evaluator = self._write_evaluator(root, contract)
        lock = write_gate_oracle_lock(root, PROJECT_ROOT)
        baseline = write_metrics(
            contract,
            corpus,
            sha256_file(evaluator),
            sha256_file(lock),
            root,
        )
        baseline_value = read_object(baseline)
        bindings = object_value(baseline_value, "bindings")
        oracle_capture = root / string_value(
            object_value(bindings, "oracle_capture"), "path"
        )
        candidate_capture = root / string_value(
            object_value(bindings, "candidate_capture"), "path"
        )
        reviews = self._write_reviews(root, baseline_value)
        script = self._write_command_script(root)
        commands = root / "commands.json"
        commands.write_text(
            json.dumps(
                {
                    "security": [
                        sys.executable,
                        script.as_posix(),
                        "security",
                        "{expected_outcome}",
                    ],
                    "quality": {
                        name: [sys.executable, script.as_posix(), "pass"]
                        for name in [
                            "tests",
                            "builds",
                            "diagnostics",
                            "contract_checks",
                        ]
                    },
                    "performance": [
                        sys.executable,
                        script.as_posix(),
                        performance_mode,
                    ],
                }
            ),
            encoding="utf-8",
        )
        return Fixture(
            root,
            contract,
            corpus,
            evaluator,
            lock,
            oracle_capture,
            candidate_capture,
            reviews,
            commands,
        )

    def _write_evaluator(self, root: Path, contract: Path) -> Path:
        lock = read_object(PROJECT_ROOT / "evaluate/multiformat/evaluator-lock.v1.json")
        value = {
            "schema_version": 2,
            "contract_sha256": sha256_file(contract),
            "project_revision": current_project_revision(PROJECT_ROOT),
            "python": lock["python"],
            "unicode_version": lock["unicode_version"],
            "algorithm_parameters": object_value(
                read_object(contract), "metric_parameters"
            ),
            "dependencies": lock["dependencies"],
            "files": [
                {"path": path, "sha256": sha256_file(PROJECT_ROOT / path)}
                for path in EVALUATOR_FILES
            ],
        }
        path = root / "evaluator.json"
        path.write_text(json.dumps(value, sort_keys=True), encoding="utf-8")
        return path

    def _write_reviews(
        self,
        root: Path,
        metrics: dict[str, JsonValue],
    ) -> tuple[Path, Path]:
        conformance = object_list(
            object_value(metrics, "conformance"), "units", "conformance.units"
        )
        blind = object_list(object_value(metrics, "blind"), "files", "blind.files")
        pair_ids = [string_value(unit, "unit_id") for unit in conformance]
        pair_ids.extend(
            string_value(unit, "unit_id")
            for file in blind
            for unit in object_list(file, "units", "blind.units")
        )
        paths: list[Path] = []
        for index, role in enumerate(["visual", "semantic-security"], start=1):
            path = root / f"decision-{index}.json"
            path.write_text(
                json.dumps(
                    {
                        "schema_version": 1,
                        "reviewer_id": f"reviewer-{index}",
                        "reviewer_role": role,
                        "independent": True,
                        "checklist_version": "multiformat-review-v1",
                        "pairs": [
                            {
                                "pair_id": pair_id,
                                "decision": "PASS",
                                "critical_defect": False,
                            }
                            for pair_id in pair_ids
                        ],
                    }
                ),
                encoding="utf-8",
            )
            paths.append(path)
        return paths[0], paths[1]

    @staticmethod
    def _write_command_script(root: Path) -> Path:
        path = root / "evidence-command.py"
        path.write_text(
            "import json, signal, sys\n"
            "mode=sys.argv[1]\n"
            "if mode=='block': signal.pause()\n"
            "if mode=='security': print(json.dumps({'observed_outcome':sys.argv[2],"
            "'typed_error':'ExpectedReject' if sys.argv[2]=='reject' else None,"
            "'network_isolation':'disabled','external_fetches':[],"
            "'active_content_executed':False,'within_limits':True}))\n",
            encoding="utf-8",
        )
        return path


if __name__ == "__main__":
    unittest.main()
