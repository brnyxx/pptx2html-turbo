from __future__ import annotations

import json
import subprocess
import tempfile
import unittest
from contextlib import redirect_stdout
from dataclasses import replace
from io import StringIO
from pathlib import Path
from unittest.mock import patch

from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

from evaluate.materialize_multiformat_candidate_runtime_locks import (
    CandidateRuntimeLockInputs,
    materialize_candidate_runtime_locks,
)
from evaluate.materialize_multiformat_candidate_runtime_locks_cli import (
    main,
    parse_args,
)
from evaluate.materialize_multiformat_portable_locks import materialize_portable_locks
from evaluate.multiformat_portable_lock_io import (
    validate_candidate_artifacts,
    validate_candidate_locks,
)
from evaluate.multiformat_revision import current_project_revision
from evaluate.multiformat_schema import sha256_file
from evaluate.tests import test_materialize_multiformat_portable_locks as portable_test


class CandidateRuntimeLockMaterializerTests(unittest.TestCase):
    def test_deterministic_exact_locks_have_no_attestation(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            inputs = self._fixture(Path(temporary))
            with patch("importlib.metadata.version", return_value="1.62.0"):
                first = materialize_candidate_runtime_locks(inputs)
                second = materialize_candidate_runtime_locks(
                    replace(inputs, output_dir=inputs.evidence_root / "locks-2")
                )
            self.assertEqual(first[0].read_bytes(), second[0].read_bytes())
            self.assertEqual(first[1].read_bytes(), second[1].read_bytes())
            browser = json.loads(first[0].read_text())
            runtime = json.loads(first[1].read_text())
            self.assertEqual(
                set(browser),
                {
                    "chromium",
                    "executable_sha256",
                    "playwright",
                    "os",
                    "architecture",
                    "font_environment_sha256",
                    "viewport_width",
                    "viewport_height",
                    "device_scale_factor",
                    "locale",
                    "timezone",
                    "color_profile",
                    "reduced_motion",
                    "animations",
                },
            )
            self.assertEqual(runtime["schema_version"], 1)
            self.assertEqual(runtime["status"], "locked")
            self.assertEqual(runtime["browser"], browser)
            self.assertEqual(runtime["sandbox_verifier"]["algorithm"], "ed25519")
            self.assertNotIn("attestation", json.dumps(runtime))
            validate_candidate_locks(*first)

    def test_refuses_dirty_debug_escape_overwrite_and_wrong_key(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            inputs = self._fixture(Path(temporary))
            with patch("importlib.metadata.version", return_value="1.62.0"):
                materialize_candidate_runtime_locks(inputs)
                with self.assertRaisesRegex(ValueError, "already exists"):
                    materialize_candidate_runtime_locks(inputs)
                with self.assertRaisesRegex(ValueError, "escapes evidence root"):
                    materialize_candidate_runtime_locks(
                        replace(inputs, output_dir=inputs.evidence_root.parent / "bad")
                    )
                debug = inputs.project_root / "target/debug/document2html"
                debug.parent.mkdir(parents=True)
                debug.write_bytes(inputs.converter.read_bytes())
                debug.chmod(0o755)
                with self.assertRaisesRegex(ValueError, "release converter"):
                    materialize_candidate_runtime_locks(
                        replace(
                            inputs,
                            converter=debug,
                            output_dir=inputs.evidence_root / "debug",
                        )
                    )
                wrong = inputs.evidence_root / "wrong.pem"
                wrong.write_text("wrong")
                with self.assertRaisesRegex(ValueError, "Ed25519 public PEM"):
                    materialize_candidate_runtime_locks(
                        replace(
                            inputs,
                            sandbox_public_key=wrong,
                            output_dir=inputs.evidence_root / "wrong",
                        )
                    )
            (inputs.project_root / "dirty").write_text("dirty")
            with patch("importlib.metadata.version", return_value="1.62.0"):
                with self.assertRaisesRegex(Exception, "clean worktree"):
                    materialize_candidate_runtime_locks(
                        replace(inputs, output_dir=inputs.evidence_root / "dirty")
                    )

    def test_strict_runtime_shape_and_pinned_playwright(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            inputs = self._fixture(Path(temporary))
            with patch("importlib.metadata.version", return_value="1.62.0"):
                browser, runtime = materialize_candidate_runtime_locks(inputs)
            value = json.loads(runtime.read_text())
            del value["candidate_runtime"]["pdfinfo_version"]
            runtime.write_text(json.dumps(value))
            with self.assertRaisesRegex(ValueError, "incomplete"):
                validate_candidate_locks(browser, runtime)
        with tempfile.TemporaryDirectory() as temporary:
            inputs = self._fixture(Path(temporary))
            with patch("importlib.metadata.version", return_value="1.61.0"):
                with self.assertRaisesRegex(ValueError, "Playwright version"):
                    materialize_candidate_runtime_locks(inputs)

    def test_tampered_hash_version_revision_font_and_extra_fields_fail(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            inputs = self._fixture(Path(temporary))
            with patch("importlib.metadata.version", return_value="1.62.0"):
                browser_path, runtime_path = materialize_candidate_runtime_locks(inputs)
            original_runtime = json.loads(runtime_path.read_text())
            original_browser = json.loads(browser_path.read_text())
            paths = {
                "browser-lock": browser_path,
                "chromium": inputs.chromium,
                "converter": inputs.converter,
                "libreoffice": inputs.soffice,
                "pdftohtml": inputs.pdftohtml,
                "poppler-metadata": inputs.pdfinfo,
                "receipt-signer": inputs.receipt_signer,
                "candidate-sandbox-public-key": inputs.sandbox_public_key,
                "openssl": inputs.openssl,
                "font-bundle": inputs.font_bundle,
            }
            versions = {
                "chromium": "Chromium 1.0",
                "converter": "converter 1.0",
                "libreoffice": "soffice 1.0",
                "pdftohtml": "pdftohtml 1.0",
                "poppler-metadata": "pdfinfo 1.0",
                "receipt-signer": "receipt-signer 1.0",
            }
            revision = current_project_revision(inputs.project_root)
            for field, value in (
                ("converter_sha256", "0" * 64),
                ("converter_version", "wrong"),
                ("build_revision", "0" * 40),
            ):
                changed = json.loads(json.dumps(original_runtime))
                changed["candidate_runtime"][field] = value
                runtime_path.write_text(json.dumps(changed))
                with self.assertRaises(ValueError):
                    validate_candidate_artifacts(
                        runtime_path, paths, versions, revision
                    )
            changed = json.loads(json.dumps(original_runtime))
            changed["unexpected"] = True
            runtime_path.write_text(json.dumps(changed))
            with self.assertRaisesRegex(ValueError, "incomplete"):
                validate_candidate_locks(browser_path, runtime_path)
            runtime_path.write_text(json.dumps(original_runtime))
            browser = json.loads(json.dumps(original_browser))
            browser["font_environment_sha256"] = "0" * 64
            browser_path.write_text(json.dumps(browser))
            with self.assertRaisesRegex(ValueError, "browser runtime lock differs"):
                validate_candidate_artifacts(runtime_path, paths, versions, revision)

    def test_handoff_into_portable_outer_lock_materializer(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            candidate = self._fixture(root / "candidate")
            outer = portable_test.PortableLockMaterializerTests()._fixture(
                root / "outer"
            )
            with patch("importlib.metadata.version", return_value="1.62.0"):
                browser, runtime = materialize_candidate_runtime_locks(candidate)
            portable = replace(
                outer,
                project_root=candidate.project_root,
                browser_lock=browser,
                candidate_runtime_lock=runtime,
                converter=candidate.converter,
                libreoffice=candidate.soffice,
                pdftohtml=candidate.pdftohtml,
                pdfinfo=candidate.pdfinfo,
                chromium=candidate.chromium,
                font_bundle=candidate.font_bundle,
                openssl=candidate.openssl,
                receipt_signer=candidate.receipt_signer,
                candidate_sandbox_public_key=candidate.sandbox_public_key,
            )
            self.assertEqual(len(materialize_portable_locks(portable)), 1)

    def test_cli_help_and_bad_input(self) -> None:
        with self.assertRaises(SystemExit) as help_exit:
            parse_args(["--help"])
        self.assertEqual(help_exit.exception.code, 0)
        with self.assertRaises(SystemExit) as bad_exit:
            parse_args([])
        self.assertEqual(bad_exit.exception.code, 2)

    def test_cli_dirty_worktree_returns_machine_readable_failure(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            inputs = self._fixture(Path(temporary))
            (inputs.project_root / "dirty").write_text("dirty")
            output = StringIO()
            argv = [
                "--project-root",
                inputs.project_root.as_posix(),
                "--evidence-root",
                inputs.evidence_root.as_posix(),
                "--output-dir",
                inputs.output_dir.as_posix(),
                "--converter",
                inputs.converter.as_posix(),
                "--soffice",
                inputs.soffice.as_posix(),
                "--pdftohtml",
                inputs.pdftohtml.as_posix(),
                "--pdfinfo",
                inputs.pdfinfo.as_posix(),
                "--receipt-signer",
                inputs.receipt_signer.as_posix(),
                "--chromium",
                inputs.chromium.as_posix(),
                "--font-bundle",
                inputs.font_bundle.as_posix(),
                "--sandbox-public-key",
                inputs.sandbox_public_key.as_posix(),
                "--openssl",
                inputs.openssl.as_posix(),
                "--verifier-id",
                inputs.verifier_id,
            ]
            with (
                patch("importlib.metadata.version", return_value="1.62.0"),
                redirect_stdout(output),
            ):
                result = main(argv)

            self.assertEqual(result, 1)
            self.assertEqual(
                json.loads(output.getvalue()),
                {
                    "status": "FAIL",
                    "reason": "candidate capture requires a clean worktree",
                },
            )
            self.assertFalse(inputs.output_dir.exists())

    def _fixture(self, root: Path) -> CandidateRuntimeLockInputs:
        project = root / "project"
        project.mkdir(parents=True)
        subprocess.run(["git", "init", "-q"], cwd=project, check=True)
        subprocess.run(
            ["git", "config", "user.email", "test@example.invalid"],
            cwd=project,
            check=True,
        )
        subprocess.run(["git", "config", "user.name", "Test"], cwd=project, check=True)
        (project / "tracked").write_text("tracked")
        subprocess.run(["git", "add", "tracked"], cwd=project, check=True)
        subprocess.run(["git", "commit", "-qm", "fixture"], cwd=project, check=True)
        evidence = project / "evidence"
        evidence.mkdir()
        release = project / "target/release"
        release.mkdir(parents=True)
        tools = {
            "converter": self._tool(release / "document2html", "converter 1.0"),
            "soffice": self._tool(root / "soffice", "soffice 1.0"),
            "pdftohtml": self._tool(root / "pdftohtml", "pdftohtml 1.0"),
            "pdfinfo": self._tool(root / "pdfinfo", "pdfinfo 1.0"),
            "receipt": self._tool(root / "receipt-signer", "receipt-signer 1.0"),
            "chromium": self._tool(root / "chromium", "Chromium 1.0"),
            "openssl": self._tool(root / "openssl", "OpenSSL 1.0"),
        }
        subprocess.run(
            ["git", "add", "target/release/document2html"], cwd=project, check=True
        )
        subprocess.run(
            ["git", "commit", "-qm", "release fixture"], cwd=project, check=True
        )
        font = evidence / "font.ttf"
        font.write_bytes(b"font")
        manifest = evidence / "font-bundle.json"
        manifest.write_text(
            json.dumps(
                {
                    "schema_version": 1,
                    "fonts": [{"path": "font.ttf", "sha256": sha256_file(font)}],
                }
            )
        )
        key = evidence / "candidate-public.pem"
        key.write_bytes(
            Ed25519PrivateKey.generate()
            .public_key()
            .public_bytes(
                serialization.Encoding.PEM,
                serialization.PublicFormat.SubjectPublicKeyInfo,
            )
        )
        return CandidateRuntimeLockInputs(
            project,
            evidence,
            evidence / "locks",
            tools["converter"],
            tools["soffice"],
            tools["pdftohtml"],
            tools["pdfinfo"],
            tools["receipt"],
            tools["chromium"],
            manifest,
            key,
            tools["openssl"],
            "candidate-sandbox-v1",
        )

    @staticmethod
    def _tool(path: Path, version: str) -> Path:
        path.write_text(f"#!/bin/sh\necho '{version}'\n")
        path.chmod(0o755)
        return path


if __name__ == "__main__":
    unittest.main()
