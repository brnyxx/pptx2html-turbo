from __future__ import annotations

import contextlib
import json
import os
import shutil
import subprocess
import tempfile
import unittest
from collections.abc import Iterator
from pathlib import Path
from unittest import mock

from evaluate.check_exactness_contract import (
    DOCUMENT_PATHS,
    WORKFLOW_PATHS,
    _bounded_child,
    check_exactness_contract,
    render_capability_matrix,
)

ROOT = Path(__file__).resolve().parents[2]


class CheckExactnessContractTests(unittest.TestCase):
    def test_repository_contract_is_clean(self) -> None:
        payload = check_exactness_contract(ROOT)
        self.assertTrue(payload["ok"])
        self.assertEqual(payload["missing_checks"], [])
        self.assertTrue(payload["generated_completion"]["relative_sha256_match"])
        self.assertTrue(payload["generated_completion"]["unique_deck_sha256"])
        self.assertEqual(payload["checked_docs"], list(DOCUMENT_PATHS))
        self.assertEqual(payload["checked_workflows"], list(WORKFLOW_PATHS))

    def test_disposition_or_verification_drift_requires_generated_refresh(self) -> None:
        mutations = (
            lambda row: row["current"]["semantic"].update(stage="resolved"),
            lambda row: row["verification"].update(focused_test_case="invented_case"),
        )
        for mutate in mutations:
            with self.subTest(mutate=mutate), self._overlay() as root:
                manifest = self._json(root)
                row = manifest["features"][0]
                mutate(row)
                self._write_manifest(root, manifest)
                payload = check_exactness_contract(root, verify_generated=False)
            self.assertIn("GENERATED_CAPABILITY_MATRIX_DRIFT:README.md", payload["missing_checks"])

    def test_typed_registration_rejects_generic_symbol_literal_symlink_and_status_mutations(self) -> None:
        for name, mutate, expected in (
            (
                "generic-pub",
                lambda root, row: row["verification"]["implementation"][0].update(token="pub"),
                "IMPLEMENTATION_TOKEN_MISSING:presentation",
            ),
            (
                "wrong-kind",
                lambda root, row: row["verification"]["implementation"][0].update(kind="substring"),
                "IMPLEMENTATION_KIND_INVALID:presentation",
            ),
            (
                "symlink",
                self._symlink_registration,
                "IMPLEMENTATION_PATH_UNSAFE:presentation",
            ),
            (
                "chart-tier",
                self._mutate_chart_rust_tier,
                "IMPLEMENTATION_STATUS_BINDING_MISMATCH:chart-preview-fallback",
            ),
        ):
            with self.subTest(name=name), self._overlay() as root:
                manifest = self._json(root)
                row = next(item for item in manifest["features"] if item["id"] == ("chart-preview-fallback" if name == "chart-tier" else "presentation"))
                mutate(root, row)
                self._write_manifest(root, manifest)
                payload = check_exactness_contract(root, verify_generated=False)
            self.assertTrue(any(item.startswith(expected) for item in payload["missing_checks"]), payload["missing_checks"])

    def test_focused_case_and_exemption_are_typed_and_exact(self) -> None:
        for feature_id, mutate, expected in (
            ("presentation", lambda v: v.update(focused_test_case="missing_case"), "FOCUSED_TEST_CASE_MISSING:presentation"),
            ("presentation", lambda v: v.update(scenario_exemption={"reason": "because prose", "dispositions": ["current", "target"]}), "SCENARIO_EXEMPTION_INVALID:presentation"),
            ("handout-master", lambda v: v.update(scenario_exemption={"reason": "baseline-outside-tasks-8-21", "dispositions": ["current", "target"]}), "COMPLETION_EVIDENCE_INVALID:handout-master"),
        ):
            with self.subTest(feature_id=feature_id), self._overlay() as root:
                manifest = self._json(root)
                row = next(item for item in manifest["features"] if item["id"] == feature_id)
                mutate(row["verification"])
                self._write_manifest(root, manifest)
                payload = check_exactness_contract(root, verify_generated=False)
            self.assertTrue(any(item.startswith(expected) for item in payload["missing_checks"]), payload["missing_checks"])

    def test_commented_focused_test_case_is_rejected(self) -> None:
        with self._overlay() as root:
            manifest = self._json(root)
            row = next(item for item in manifest["features"] if item["id"] == "presentation")
            row["verification"]["focused_test_case"] = "fabricated_comment_only_case"
            focused_test = root / row["verification"]["focused_test"]
            focused_test.write_text(
                focused_test.read_text()
                + "\n/*\n#[test]\nfn fabricated_comment_only_case() {}\n*/\n"
            )
            self._write_manifest(root, manifest)
            payload = check_exactness_contract(root, verify_generated=False)
        self.assertTrue(
            any(
                item.startswith("FOCUSED_TEST_CASE_MISSING:presentation:")
                for item in payload["missing_checks"]
            ),
            payload["missing_checks"],
        )

    def test_diagnostic_support_tier_rejects_typed_decoy(self) -> None:
        with self._overlay() as root:
            path = root / "crates/pptx2html-core/src/parser/chart_diagnostics.rs"
            source = path.read_text()
            source = source.replace(
                ") -> ConversionDiagnostic {\n    let _exactness_disposition",
                ") -> ConversionDiagnostic {\n"
                "    let _checker_decoy: SupportTier = SupportTier::Fallback;\n"
                "    let _exactness_disposition",
                1,
            )
            source = source.replace(
                "support_tier: SupportTier::Fallback,",
                "support_tier: SupportTier::Approximate,",
                1,
            )
            path.write_text(source)
            payload = check_exactness_contract(root, verify_generated=False)
        self.assertIn(
            "IMPLEMENTATION_STATUS_BINDING_MISMATCH:chart-preview-fallback:0",
            payload["missing_checks"],
        )

    def test_scenario_schema_and_relationship_declarations_are_required(self) -> None:
        with self._overlay() as root:
            features = root / "evaluate/completion_deck_features.py"
            features.write_text(features.read_text().replace('relationship_disposition="internal-video",', 'relationship_disposition="none",', 1))
            payload = check_exactness_contract(root)
        self.assertTrue(
            "MEDIA_VIDEO_INTERNAL_FIXTURE_REQUIRED" in payload["missing_checks"]
            or any(
                item.startswith("COMPLETION_VALIDATOR_NONZERO:")
                and "internal-video" in item
                for item in payload["missing_checks"]
            ),
            payload["missing_checks"],
        )

    def test_workflow_comments_do_not_count_and_real_gate_after_publish_fails(self) -> None:
        with self._overlay() as root:
            workflow = root / ".github/workflows/publish-npm.yml"
            source = workflow.read_text()
            source = source.replace(
                "        run: python3 evaluate/check_exactness_contract.py --repo-root .",
                "        run: |\n          # python3 evaluate/check_exactness_contract.py --repo-root .\n          python3 -c 'pass'",
                1,
            )
            source += "\n      - name: Too late\n        run: python3 evaluate/check_exactness_contract.py --repo-root .\n"
            workflow.write_text(source)
            payload = check_exactness_contract(root, verify_generated=False)
        self.assertIn("WORKFLOW_GATE_ORDER_INVALID:.github/workflows/publish-npm.yml", payload["missing_checks"])

    def test_disabled_workflow_gates_do_not_satisfy_publication_order(self) -> None:
        with self._overlay() as root:
            workflow = root / ".github/workflows/publish-npm.yml"
            source = workflow.read_text()
            source = source.replace(
                "        run: python3 -m unittest discover -s evaluate/tests -p 'test_*.py' -v",
                "        if: ${{ false }}\n"
                "        run: python3 -m unittest discover -s evaluate/tests -p 'test_*.py' -v",
                1,
            )
            source = source.replace(
                "        run: python3 evaluate/check_exactness_contract.py --repo-root .",
                "        if: ${{ false }}\n"
                "        run: python3 evaluate/check_exactness_contract.py --repo-root .",
                1,
            )
            workflow.write_text(source)
            payload = check_exactness_contract(root, verify_generated=False)
        self.assertIn(
            "WORKFLOW_GATE_ORDER_INVALID:.github/workflows/publish-npm.yml",
            payload["missing_checks"],
        )

    def test_continue_on_error_workflow_gates_do_not_protect_publication(self) -> None:
        with self._overlay() as root:
            workflow = root / ".github/workflows/publish-npm.yml"
            source = workflow.read_text()
            source = source.replace(
                "        run: python3 -m unittest discover -s evaluate/tests -p 'test_*.py' -v",
                "        continue-on-error: true\n"
                "        run: python3 -m unittest discover -s evaluate/tests -p 'test_*.py' -v",
                1,
            )
            source = source.replace(
                "        run: python3 evaluate/check_exactness_contract.py --repo-root .",
                "        continue-on-error: true\n"
                "        run: python3 evaluate/check_exactness_contract.py --repo-root .",
                1,
            )
            workflow.write_text(source)
            payload = check_exactness_contract(root, verify_generated=False)
        self.assertIn(
            "WORKFLOW_GATE_ORDER_INVALID:.github/workflows/publish-npm.yml",
            payload["missing_checks"],
        )

    def test_non_manifest_status_table_drift_is_rejected(self) -> None:
        with self._overlay() as root:
            supported = root / "SUPPORTED_FEATURES.md"
            supported.write_text(
                supported.read_text()
                + "\n | Feature | Status |\n"
                + " | --- | --- |\n"
                + " | Rectangle | Not yet |\n"
            )
            payload = check_exactness_contract(root, verify_generated=False)
        self.assertIn(
            "NON_MANIFEST_STATUS_DOCUMENTATION:SUPPORTED_FEATURES.md",
            payload["missing_checks"],
        )

    def test_child_generator_timeout_and_os_exit_are_stable(self) -> None:
        scripts = {
            "timeout": "import time\ntime.sleep(60)\n",
            "exit": "import os\nos._exit(23)\n",
        }
        for name, script in scripts.items():
            with self.subTest(name=name), self._overlay() as root:
                generator = root / "evaluate/create_completion_decks.py"
                generator.write_text(script)
                with mock.patch.dict(os.environ, {"PPTX_EXACTNESS_CHILD_TIMEOUT": "0.1"}):
                    payload = check_exactness_contract(root)
            expected = "COMPLETION_GENERATOR_1_TIMEOUT" if name == "timeout" else "COMPLETION_GENERATOR_1_NONZERO:23:"
            self.assertTrue(any(item.startswith(expected) for item in payload["missing_checks"]), payload["missing_checks"])

    @unittest.skipUnless(os.name == "posix", "process-group assertion requires POSIX signals")
    def test_bounded_child_terminates_timed_out_descendants(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            script = root / "spawn_descendant.py"
            pid_path = root / "descendant.pid"
            script.write_text(
                "import pathlib\n"
                "import signal\n"
                "import subprocess\n"
                "import sys\n"
                "child = subprocess.Popen([sys.executable, '-c', "
                "'import signal; signal.pause()'])\n"
                f"pathlib.Path({str(pid_path)!r}).write_text(str(child.pid))\n"
                "signal.pause()\n"
            )
            with mock.patch.dict(os.environ, {"PPTX_EXACTNESS_CHILD_TIMEOUT": "0.1"}):
                ok, _ = _bounded_child(
                    [os.fspath(Path(os.sys.executable)), os.fspath(script)],
                    root,
                    "DESCENDANT",
                )
            self.assertFalse(ok)
            descendant_pid = int(pid_path.read_text())
            try:
                os.kill(descendant_pid, 0)
            except ProcessLookupError:
                alive = False
            else:
                state = subprocess.run(
                    ("ps", "-o", "stat=", "-p", str(descendant_pid)),
                    capture_output=True,
                    check=False,
                    text=True,
                )
                alive = state.returncode == 0 and not state.stdout.lstrip().startswith("Z")
                if alive:
                    os.kill(descendant_pid, 9)
            self.assertFalse(alive, "timed-out subprocess descendant survived")

    def test_duplicate_generated_deck_bytes_are_rejected(self) -> None:
        with self._overlay() as root:
            generator = root / "evaluate/create_completion_decks.py"
            generator.write_text(
                "from pathlib import Path\nimport argparse\n"
                "p=argparse.ArgumentParser(); p.add_argument('--output-dir',type=Path); p.add_argument('--adjustment-manifest'); a=p.parse_args(); a.output_dir.mkdir(); "
                "(a.output_dir/'a.pptx').write_bytes(b'same'); (a.output_dir/'b.pptx').write_bytes(b'same'); (a.output_dir/'manifest.json').write_text('{}')\n"
            )
            payload = check_exactness_contract(root)
        self.assertIn("COMPLETION_DECK_SHA256_DUPLICATE", payload["missing_checks"])

    def test_exact_tier_rejects_secondary_or_missing_native_provenance(self) -> None:
        with self._overlay() as root:
            manifest = self._json(root)
            row = manifest["features"][0]
            row["current"]["semantic"] = {"tier": "exact", "stage": "fidelity-tested"}
            row["exact_evidence"] = {"oracle": "PowerPoint-native", "producer": "LibreOffice"}
            self._write_manifest(root, manifest)
            payload = check_exactness_contract(root, verify_generated=False)
        self.assertTrue(any("EXACT_REQUIRES_POWERPOINT_EVIDENCE" in item for item in payload["missing_checks"]), payload["missing_checks"])
        self.assertIn("EXACT_TIER_WITHOUT_READY_NATIVE_TEXT_LAYOUT_GATE", payload["missing_checks"])

    def test_rendered_matrix_hashes_full_status_and_verification(self) -> None:
        manifest = json.loads((ROOT / "evaluate/completeness_manifest.json").read_text())
        block, digest = render_capability_matrix(manifest)
        self.assertIn(f"manifest-sha256: {digest}", block)
        self.assertEqual(block.count("<a id=\"capability-"), 56)
        self.assertIn("Verification SHA256", block)
        self.assertIn("Target S/V/B", block)

    @contextlib.contextmanager
    def _overlay(self) -> Iterator[Path]:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for name in ("README.md", "SUPPORTED_FEATURES.md", "docs", "evaluate", "crates", ".github"):
                source = ROOT / name
                if source.is_dir():
                    shutil.copytree(source, root / name, ignore=shutil.ignore_patterns("__pycache__"))
                else:
                    shutil.copy2(source, root / name)
            yield root

    def _json(self, root: Path) -> dict[str, object]:
        return json.loads((root / "evaluate/completeness_manifest.json").read_text())

    def _write_manifest(self, root: Path, payload: dict[str, object]) -> None:
        (root / "evaluate/completeness_manifest.json").write_text(json.dumps(payload, indent=2) + "\n")

    def _symlink_registration(self, root: Path, row: dict[str, object]) -> None:
        link = root / "crates/pptx2html-core/src/model/link.rs"
        link.symlink_to(link.with_name("capabilities.rs"))
        row["verification"]["implementation"][0]["path"] = "crates/pptx2html-core/src/model/link.rs"

    def _mutate_chart_rust_tier(self, root: Path, row: dict[str, object]) -> None:
        path = root / "crates/pptx2html-core/src/parser/chart_diagnostics.rs"
        path.write_text(path.read_text().replace("SupportTier::Fallback", "SupportTier::Approximate", 1))


if __name__ == "__main__":
    unittest.main()
