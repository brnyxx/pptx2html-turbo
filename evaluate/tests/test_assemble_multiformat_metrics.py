from __future__ import annotations

import ast
import json
import os
import sys
import tempfile
import unittest
from dataclasses import dataclass
from pathlib import Path
from typing import cast

from evaluate.assemble_multiformat_metrics import assemble_metrics
from evaluate.assemble_multiformat_report import assemble_report
from evaluate.multiformat_command_evidence import (
    CommandEvidenceError,
    CommandPlan,
    run_performance_command,
)
from evaluate.multiformat_corpus_items import object_list
from evaluate.multiformat_evaluator_files import EVALUATOR_FILES
from evaluate.multiformat_metric_manifest import MetricsAssemblyError
from evaluate.multiformat_metric_types import MetricError
from evaluate.multiformat_metrics import validate_metrics_evidence
from evaluate.multiformat_revision import current_project_revision
from evaluate.multiformat_schema import (
    JsonValue,
    object_value,
    read_object,
    sha256_file,
    sha256_value,
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
            self.assertFalse((fixture.root / "executions-metrics-produced").exists())

    def test_duplicate_reviewer_identity_fails(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            fixture = self._fixture(Path(temporary))
            second = read_object(fixture.reviews[1])
            second["reviewer_id"] = "reviewer-1"
            fixture.reviews[1].write_text(json.dumps(second), encoding="utf-8")

            with self.assertRaises(MetricsAssemblyError):
                self._assemble(fixture)

    def test_security_nonzero_and_invalid_json_fail_and_clean_output(self) -> None:
        for mode in ["fail", "invalid"]:
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as temporary:
                fixture = self._fixture(Path(temporary), security_mode=mode)
                execution = fixture.root / "executions-metrics-produced"
                with self.assertRaises(CommandEvidenceError):
                    self._assemble(fixture)
                self.assertFalse(execution.exists())

    def test_security_output_is_bounded(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            fixture = self._fixture(Path(temporary), security_mode="flood")
            with self.assertRaises(CommandEvidenceError):
                self._assemble(fixture)
            self.assertFalse((fixture.root / "executions-metrics-produced").exists())

    def test_security_missing_and_extra_case_coverage_fails(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            fixture = self._fixture(Path(temporary))
            output = self._assemble(fixture)
            for mode in ["missing", "extra"]:
                metrics = read_object(output)
                security = object_value(metrics, "security")
                cases = object_list(security, "cases", "security.cases")
                if mode == "missing":
                    cases.pop()
                else:
                    cases.append(dict(cases[0]))
                security["cases"] = cast(list[JsonValue], cases)
                path = fixture.root / f"metrics-{mode}.json"
                path.write_text(json.dumps(metrics), encoding="utf-8")
                with self.assertRaises(MetricError):
                    self._validate(fixture, path)

    def test_quality_nonzero_is_derived_as_failure(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            fixture = self._fixture(Path(temporary), quality_failure="tests")
            summary = self._validate(fixture, self._assemble(fixture))
            self.assertFalse(summary.quality.tests_passed)
            self.assertTrue(summary.quality.builds_passed)

    def test_performance_nonzero_is_derived_as_failure(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            fixture = self._fixture(Path(temporary), performance_mode="fail")
            summary = self._validate(fixture, self._assemble(fixture))
            self.assertFalse(summary.performance_within_limits)

    def test_commands_do_not_inherit_ambient_environment(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            fixture = self._fixture(Path(temporary), quality_failure="env-check")
            previous = os.environ.get("ASSEMBLER_SECRET")
            os.environ["ASSEMBLER_SECRET"] = "must-not-leak"
            try:
                summary = self._validate(fixture, self._assemble(fixture))
            finally:
                if previous is None:
                    os.environ.pop("ASSEMBLER_SECRET", None)
                else:
                    os.environ["ASSEMBLER_SECRET"] = previous
            self.assertTrue(summary.quality.tests_passed)

    def test_each_command_gets_a_fresh_home_and_temporary_directory(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            fixture = self._fixture(Path(temporary), quality_isolation=True)
            summary = self._validate(fixture, self._assemble(fixture))
            self.assertTrue(summary.quality.builds_passed)
            self.assertTrue(summary.quality.contract_checks_passed)

    def test_temporary_environment_is_removed_on_timeout_but_logs_remain(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            script = self._write_command_script(root)
            output = root / "runtime"
            plan = CommandPlan((), {}, (sys.executable, script.as_posix(), "block"))
            with self.assertRaises(CommandEvidenceError):
                run_performance_command(
                    plan,
                    root,
                    output,
                    bindings={
                        "project_revision": "revision",
                        "evaluator_hash": "e" * 64,
                        "corpus_hash": "c" * 64,
                    },
                    working_directory=PROJECT_ROOT,
                    timeout_seconds=1,
                )
            self.assertTrue((output / "performance.stdout").is_file())
            self.assertTrue((output / "performance.stderr").is_file())
            self.assertFalse(
                any(path.name.startswith(".") for path in output.iterdir())
            )

    def test_reviewer_missing_and_extra_pair_fail(self) -> None:
        for mode in ["missing", "extra"]:
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as temporary:
                fixture = self._fixture(Path(temporary))
                review = read_object(fixture.reviews[0])
                pairs = object_list(review, "pairs", "review.pairs")
                if mode == "missing":
                    pairs.pop()
                else:
                    pairs.append(
                        {
                            "pair_id": "extra-pair",
                            "decision": "PASS",
                            "critical_defect": False,
                        }
                    )
                review["pairs"] = cast(list[JsonValue], pairs)
                fixture.reviews[0].write_text(json.dumps(review), encoding="utf-8")
                with self.assertRaises(MetricsAssemblyError):
                    self._assemble(fixture)

    def test_reviewer_roles_must_be_distinct(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            fixture = self._fixture(Path(temporary))
            first = read_object(fixture.reviews[0])
            second = read_object(fixture.reviews[1])
            second["reviewer_role"] = first["reviewer_role"]
            fixture.reviews[1].write_text(json.dumps(second), encoding="utf-8")
            with self.assertRaises(MetricsAssemblyError):
                self._assemble(fixture)

    def test_review_hashes_are_derived_from_captures(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            fixture = self._fixture(Path(temporary))
            metrics = read_object(self._assemble(fixture))
            reviewer = object_list(
                object_value(metrics, "review"), "reviewers", "review.reviewers"
            )[0]
            attestation = object_value(reviewer, "attestation")
            review = read_object(fixture.root / string_value(attestation, "path"))
            pair = object_list(review, "pairs", "review.pairs")[0]
            candidate = read_object(fixture.candidate_capture)
            units = {
                string_value(unit, "unit_id"): unit
                for unit in object_list(candidate, "units", "capture.units")
            }
            captured = units[string_value(pair, "pair_id")]
            self.assertEqual(
                sha256_value(pair, "candidate_png_sha256"),
                sha256_value(object_value(captured, "png"), "sha256"),
            )

    def test_critical_defects_are_ored_across_reviewers(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            fixture = self._fixture(Path(temporary))
            review = read_object(fixture.reviews[0])
            pairs = object_list(review, "pairs", "review.pairs")
            target_id = string_value(pairs[0], "pair_id")
            pairs[0]["critical_defect"] = True
            fixture.reviews[0].write_text(json.dumps(review), encoding="utf-8")
            metrics = read_object(self._assemble(fixture))
            records = object_list(
                object_value(metrics, "conformance"), "units", "conformance.units"
            )
            target = next(
                record
                for record in records
                if string_value(record, "unit_id") == target_id
            )
            self.assertIs(target["critical_defect"], True)

    def test_candidate_determinism_is_copied_exactly(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            fixture = self._fixture(Path(temporary))
            metrics = read_object(self._assemble(fixture))
            candidate = read_object(fixture.candidate_capture)
            binding = object_value(candidate, "determinism_manifest")
            expected = read_object(fixture.root / string_value(binding, "path"))
            self.assertEqual(object_value(metrics, "determinism"), expected)

    def test_existing_ready_output_is_refused_and_retry_tree_removed(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            fixture = self._fixture(Path(temporary))
            output = self._assemble(fixture)
            original = output.read_bytes()
            retry = fixture.root / "retry-executions"
            with self.assertRaises(MetricsAssemblyError):
                self._assemble(fixture, output=output, execution_dir=retry)
            self.assertEqual(output.read_bytes(), original)
            self.assertFalse(retry.exists())

    def test_preexisting_execution_content_is_never_removed(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            fixture = self._fixture(Path(temporary))
            execution = fixture.root / "preexisting"
            execution.mkdir()
            sentinel = execution / "keep.txt"
            sentinel.write_text("keep", encoding="utf-8")
            with self.assertRaises(MetricsAssemblyError):
                self._assemble(fixture, execution_dir=execution)
            self.assertEqual(sentinel.read_text(encoding="utf-8"), "keep")

    def test_production_modules_do_not_import_test_helpers(self) -> None:
        for name in [
            "assemble_multiformat_metrics.py",
            "multiformat_command_evidence.py",
            "multiformat_metric_manifest.py",
            "multiformat_review_materialize.py",
        ]:
            path = PROJECT_ROOT / "evaluate" / name
            tree = ast.parse(path.read_text(encoding="utf-8"))
            imports = [
                node.module or ""
                for node in ast.walk(tree)
                if isinstance(node, ast.ImportFrom)
            ]
            self.assertFalse(
                any(value.startswith("evaluate.tests") for value in imports)
            )

    def _assemble(
        self,
        fixture: Fixture,
        *,
        output: Path | None = None,
        timeout: int = 10,
        execution_dir: Path | None = None,
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
            execution_output_dir=(
                execution_dir or fixture.root / f"executions-{target.stem}"
            ),
            output_path=target,
            timeout_seconds=timeout,
        )
        return target

    def _fixture(
        self,
        root: Path,
        *,
        security_mode: str = "security",
        quality_failure: str | None = None,
        quality_isolation: bool = False,
        performance_mode: str = "pass",
    ) -> Fixture:
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
                        security_mode,
                        "{expected_outcome}",
                    ],
                    "quality": {
                        name: [
                            sys.executable,
                            script.as_posix(),
                            (
                                "leave-marker"
                                if quality_isolation and name == "builds"
                                else "detect-marker"
                                if quality_isolation and name == "contract_checks"
                                else "env-check"
                                if quality_failure == "env-check" and name == "tests"
                                else "fail"
                                if quality_failure == name
                                else "pass"
                            ),
                        ]
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

    def _validate(self, fixture: Fixture, metrics: Path):
        return validate_metrics_evidence(
            fixture.contract,
            fixture.corpus,
            metrics,
            sha256_file(fixture.evaluator),
            sha256_file(fixture.lock),
            fixture.root,
            fixture.lock,
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
            "import json, os, signal, sys\n"
            "mode=sys.argv[1]\n"
            "if mode=='block': signal.pause()\n"
            "if mode=='fail': sys.exit(7)\n"
            "if mode=='invalid': print('not-json')\n"
            "if mode=='flood': print('x'*(9*1024*1024))\n"
            "if mode=='env-check': sys.exit(8 if 'ASSEMBLER_SECRET' in os.environ else 0)\n"
            "if mode=='leave-marker': open(os.path.join(os.environ['HOME'],'marker'),'w').write('x')\n"
            "if mode=='detect-marker': sys.exit(9 if os.path.exists(os.path.join(os.environ['HOME'],'marker')) else 0)\n"
            "if mode=='security': print(json.dumps({'observed_outcome':sys.argv[2],"
            "'typed_error':'ExpectedReject' if sys.argv[2]=='reject' else None,"
            "'network_isolation':'disabled','external_fetches':[],"
            "'active_content_executed':False,'within_limits':True}))\n",
            encoding="utf-8",
        )
        return path


if __name__ == "__main__":
    unittest.main()
