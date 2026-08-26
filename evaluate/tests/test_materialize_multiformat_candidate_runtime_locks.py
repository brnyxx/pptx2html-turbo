from __future__ import annotations

import json
import tempfile
import unittest
from dataclasses import replace
from pathlib import Path
from unittest.mock import patch

from evaluate.materialize_multiformat_candidate_runtime_locks import (
    CandidateRuntimeLockInputs,
    materialize_candidate_runtime_locks,
)
from evaluate.materialize_multiformat_portable_locks import materialize_portable_locks
from evaluate.multiformat_portable_lock_io import (
    validate_candidate_artifacts,
    validate_candidate_locks,
)
from evaluate.multiformat_revision import current_project_revision
from evaluate.tests import test_materialize_multiformat_portable_locks as portable_test
from evaluate.tests.multiformat_candidate_runtime_lock_fixture import (
    candidate_runtime_lock_inputs,
)


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

    def test_multiline_tool_banners_bind_the_first_version_line(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            inputs = self._fixture(Path(temporary))
            inputs.pdftohtml.write_text(
                "#!/bin/sh\n"
                "printf 'pdftohtml version 26.03.0\\nCopyright Poppler\\n' >&2\n"
            )
            inputs.pdfinfo.write_text(
                "#!/bin/sh\n"
                "printf 'pdfinfo version 26.03.0\\nCopyright Poppler\\n' >&2\n"
            )
            with patch("importlib.metadata.version", return_value="1.62.0"):
                _, runtime_path = materialize_candidate_runtime_locks(inputs)

            runtime = json.loads(runtime_path.read_text())
            candidate = runtime["candidate_runtime"]
            self.assertEqual(
                candidate["pdftohtml_version"],
                "pdftohtml version 26.03.0",
            )
            self.assertEqual(
                candidate["pdfinfo_version"],
                "pdfinfo version 26.03.0",
            )

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
                chromium=candidate.chromium,
                font_bundle=candidate.font_bundle,
                receipt_signer=candidate.receipt_signer,
                candidate_sandbox_public_key=candidate.sandbox_public_key,
            )
            self.assertEqual(len(materialize_portable_locks(portable)), 1)

    def _fixture(self, root: Path) -> CandidateRuntimeLockInputs:
        return candidate_runtime_lock_inputs(root)


if __name__ == "__main__":
    unittest.main()
