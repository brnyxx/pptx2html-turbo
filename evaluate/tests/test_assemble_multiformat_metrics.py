from __future__ import annotations

import ast
import hashlib
import json
import os
import subprocess
import sys
import tempfile
import unittest
from dataclasses import dataclass
from pathlib import Path
from typing import cast
from unittest import mock

from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

from evaluate.assemble_multiformat_metrics import assemble_metrics
from evaluate.assemble_multiformat_report import assemble_report
from evaluate.materialize_multiformat_command_plan import materialize_command_plan
from evaluate.multiformat_command_evidence import CommandEvidenceError
from evaluate.multiformat_command_runtime import _run_command
from evaluate.multiformat_corpus_items import object_list
from evaluate.multiformat_evaluator_files import EVALUATOR_FILES
from evaluate.multiformat_metric_manifest import (
    MetricsAssemblyError,
    prepare_metric_context,
)
from evaluate.multiformat_metric_types import MetricError
from evaluate.multiformat_metrics import validate_metrics_evidence
from evaluate.multiformat_review_packet import materialize_review_packet
from evaluate.multiformat_review_registry import (
    RegisteredReviewer,
    ReviewerRegistry,
)
from evaluate.multiformat_revision import current_project_revision
from evaluate.multiformat_schema import (
    JsonValue,
    object_value,
    read_object,
    sha256_file,
    sha256_value,
    string_list,
    string_value,
)
from evaluate.sign_multiformat_review_decision import sign_review_decision
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
    review_packet: Path
    reviews: tuple[Path, Path]
    review_keys: tuple[Path, Path]
    commands: Path
    security_mode: str
    quality_failure: str | None
    performance_mode: str


class AssembleMultiformatMetricsTests(unittest.TestCase):
    # Reviewer identities used by this test's generated keypairs. Trust is
    # resolved through a test registry built from those exact public keys, so
    # the tracked production registry is never consulted here.
    REVIEWERS = (
        ("reviewer-1", "visual"),
        ("reviewer-2", "semantic-security"),
    )

    def _patch_registry(self, public_keys: tuple[bytes, bytes]) -> None:
        registry = ReviewerRegistry(
            tuple(
                RegisteredReviewer(
                    reviewer_id,
                    role,
                    key,
                    hashlib.sha256(key).hexdigest(),
                )
                for (reviewer_id, role), key in zip(
                    self.REVIEWERS, public_keys, strict=True
                )
            )
        )
        for target in (
            "evaluate.multiformat_review_packet.load_reviewer_registry",
            "evaluate.multiformat_review_packet_trust.load_reviewer_registry",
            "evaluate.multiformat_metric_review.load_reviewer_registry",
        ):
            self.enterContext(mock.patch(target, return_value=registry))

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

    def test_command_plan_cannot_substitute_a_different_outer_lock(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            fixture = self._fixture(Path(temporary))
            foreign_lock = fixture.root / "foreign-lock.json"
            foreign = read_object(fixture.lock)
            foreign["foreign"] = True
            foreign_lock.write_text(json.dumps(foreign), encoding="utf-8")
            commands = read_object(fixture.commands)
            outer_lock = object_value(commands, "outer_lock")
            outer_lock["path"] = foreign_lock.name
            outer_lock["sha256"] = sha256_file(foreign_lock)
            fixture.commands.write_text(json.dumps(commands), encoding="utf-8")

            with self.assertRaises(MetricsAssemblyError):
                self._assemble(fixture)

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
            with self.assertRaises((MetricError, OSError, TypeError, ValueError)):
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
            python = Path(sys.executable).resolve()
            with self.assertRaises(CommandEvidenceError):
                _run_command(
                    (python.as_posix(), script.as_posix(), "block"),
                    {},
                    output / "performance.stdout",
                    output / "performance.stderr",
                    PROJECT_ROOT,
                    1,
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

    def test_blank_reviewer_template_is_rejected_by_metrics(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            fixture = self._fixture(Path(temporary))
            review = read_object(fixture.reviews[0])
            pairs = object_list(review, "pairs", "review.pairs")
            for pair in pairs:
                pair["decision"] = None
                pair["critical_defect"] = None
            fixture.reviews[0].write_text(json.dumps(review), encoding="utf-8")
            with self.assertRaises((MetricsAssemblyError, TypeError, ValueError)):
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
            review = read_object(fixture.review_packet)
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
            unsigned = fixture.root / "critical-unsigned.json"
            review.pop("signature")
            unsigned.write_text(json.dumps(review), encoding="utf-8")
            fixture.reviews[0].unlink()
            sign_review_decision(unsigned, fixture.review_keys[0], fixture.reviews[0])
            metrics = read_object(self._assemble(fixture))
            records = object_list(
                object_value(metrics, "conformance"), "units", "conformance.units"
            )
            for file in object_list(
                object_value(metrics, "blind"), "files", "blind.files"
            ):
                records.extend(object_list(file, "units", "blind.units"))
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

        def security_result(*args, **kwargs):
            if fixture.security_mode != "security":
                raise CommandEvidenceError("security command returned invalid evidence")
            substitutions = args[1]
            expected = substitutions["expected_outcome"]
            return {
                "observed_outcome": expected,
                "typed_error": "ExpectedReject" if expected == "reject" else None,
                "network_isolation": "disabled",
                "external_fetches": [],
                "active_content_executed": False,
                "within_limits": True,
            }

        def command_result(argv, *args, **kwargs):
            if "evaluate.check_exactness_contract" in argv:
                role = "contract_checks"
            elif "clippy" in argv:
                role = "diagnostics"
            elif "build" in argv:
                role = "builds"
            elif "--release" in argv:
                if fixture.performance_mode == "block":
                    raise CommandEvidenceError("command timed out")
                return 7 if fixture.performance_mode == "fail" else 0
            else:
                role = "tests"
            return 7 if fixture.quality_failure == role else 0

        with (
            mock.patch(
                "evaluate.multiformat_command_runtime._run_json_command",
                side_effect=security_result,
            ),
            mock.patch(
                "evaluate.multiformat_command_runtime._run_command",
                side_effect=command_result,
            ),
        ):
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
                review_packet_path=fixture.review_packet,
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
            lock,
        )
        baseline_value = read_object(baseline)
        bindings = object_value(baseline_value, "bindings")
        oracle_capture = root / string_value(
            object_value(bindings, "oracle_capture"), "path"
        )
        candidate_capture = root / string_value(
            object_value(bindings, "candidate_capture"), "path"
        )
        context = prepare_metric_context(
            PROJECT_ROOT,
            contract,
            corpus,
            evaluator,
            lock,
            oracle_capture,
            candidate_capture,
            root.resolve(strict=True),
        )
        review_packet, reviews, review_keys = self._write_reviews(root, context)
        commands = root / "commands.json"
        python = Path(sys.executable).resolve().as_posix()
        cargo = subprocess.run(
            ["rustup", "which", "cargo"],
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()
        rustc = subprocess.run(
            ["rustup", "which", "rustc"],
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()
        env = Path("/usr/bin/env").resolve().as_posix()
        path_arg = f"PATH={Path(rustc).parent}:/usr/bin:/bin"
        quality = {
            "tests": (
                env,
                path_arg,
                cargo,
                "test",
                "-p",
                "document2html-core",
                "-p",
                "document2html-native",
            ),
            "builds": (
                env,
                path_arg,
                cargo,
                "build",
                "--release",
                "-p",
                "pptx2html-cli",
                "--bin",
                "document2html",
            ),
            "diagnostics": (
                env,
                path_arg,
                cargo,
                "clippy",
                "-p",
                "document2html-core",
                "-p",
                "document2html-native",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ),
            "contract_checks": (
                python,
                "-m",
                "evaluate.check_exactness_contract",
                "--repo-root",
                PROJECT_ROOT.as_posix(),
            ),
        }
        materialize_command_plan(
            commands,
            (python, "-m", "evaluate.run_multiformat_security_case"),
            quality,
            (env, path_arg, cargo, "test", "--release", "-p", "document2html-native"),
            outer_lock=lock,
        )
        return Fixture(
            root,
            contract,
            corpus,
            evaluator,
            lock,
            oracle_capture,
            candidate_capture,
            review_packet,
            reviews,
            review_keys,
            commands,
            security_mode,
            quality_failure,
            performance_mode,
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
        self, root: Path, context
    ) -> tuple[Path, tuple[Path, Path], tuple[Path, Path]]:
        private_paths: list[Path] = []
        public_bytes: list[bytes] = []
        for index in range(2):
            key = Ed25519PrivateKey.generate()
            private = root / f"review-private-{index}.key"
            private.write_bytes(key.private_bytes_raw())
            os.chmod(private, 0o600)
            private_paths.append(private)
            public_bytes.append(key.public_key().public_bytes_raw())
        self._patch_registry((public_bytes[0], public_bytes[1]))
        review_root = root / "review"
        summary = materialize_review_packet(
            review_root,
            context.oracle,
            context.candidate,
            context.spec.pair_ids(),
            bindings={
                "project_revision": context.project_revision,
                "contract_sha256": context.contract_hash,
                "corpus_manifest_sha256": context.corpus_hash,
                "evaluator_manifest_sha256": context.evaluator_hash,
                "oracle_lock_sha256": context.oracle_hash,
                "oracle_capture": context.oracle_binding,
                "candidate_capture": context.candidate_binding,
            },
        )
        signed: list[Path] = []
        for index, template_value in enumerate(
            string_list(summary, "decision_templates")
        ):
            template = Path(template_value)
            value = read_object(template)
            for pair in object_list(value, "pairs", "review.pairs"):
                pair["decision"] = "PASS"
                pair["critical_defect"] = False
            template.write_text(json.dumps(value), encoding="utf-8")
            output = root / f"decision-{index + 1}.json"
            sign_review_decision(template, private_paths[index], output)
            signed.append(output)
        return (
            Path(str(summary["review_packet"])),
            (signed[0], signed[1]),
            (private_paths[0], private_paths[1]),
        )

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
