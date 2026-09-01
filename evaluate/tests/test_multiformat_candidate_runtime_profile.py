from __future__ import annotations

import json
import platform
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

from evaluate.multiformat_candidate_attestation import VerifiedAttestation
from evaluate.multiformat_candidate_preflight import preflight_candidate_capture
from evaluate.multiformat_candidate_preflight_runtime import (
    require_candidate_evidence_root,
)
from evaluate.multiformat_candidate_runtime_profile import (
    CandidateRuntimeProfileError,
    legacy_candidate_runtime_profile,
    resolve_candidate_runtime_profile,
)
from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_schema import JsonValue, sha256_file
from evaluate.tests.multiformat_portable_receipt_fixture import ReceiptFixture


class CandidateRuntimeProfileTests(unittest.TestCase):
    def test_schema_1_adapter_preserves_runtime_and_verifier_identity(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            lock = Path(temp_dir) / "lock.json"
            runtime: dict[str, JsonValue] = {
                "build_revision": "a" * 40,
                "runtime_marker": "candidate-runtime",
            }
            verifier: dict[str, JsonValue] = {
                "algorithm": "ed25519",
                "verifier_id": "schema1-sandbox",
                "verifier_marker": "sandbox-verifier",
            }
            lock.write_text(
                json.dumps(
                    {
                        "browser": {"chromium": "schema1-browser"},
                        "candidate_runtime": runtime,
                        "sandbox_verifier": verifier,
                    }
                ),
                encoding="utf-8",
            )
            profile = legacy_candidate_runtime_profile(lock)
            self.assertEqual(profile.candidate_runtime_lock, runtime)
            self.assertEqual(profile.sandbox_verifier, verifier)

    def test_candidate_attestation_outside_evidence_root_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            evidence = root / "evidence"
            evidence.mkdir()
            outside = root / "candidate-attestation.json"
            outside.write_text("{}", encoding="utf-8")
            with self.assertRaisesRegex(
                CandidateRuntimeProfileError, "escapes evidence root"
            ):
                require_candidate_evidence_root(evidence, (outside,))

    def test_schema_2_resolves_only_bound_runtime_artifacts(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            fixture = ReceiptFixture(Path(temp_dir))
            lock = self._prepare_runtime_artifacts(fixture)

            profile = resolve_candidate_runtime_profile(
                fixture.lock,
                fixture.root,
                fixture.root / "locked/contract",
                fixture.root / "corpus/manifest.json",
                fixture.root / "locked/evaluator",
                "6" * 40,
            )

            self.assertEqual(profile.schema_version, 2)
            self.assertEqual(profile.browser_version, "test-chromium")
            self.assertEqual(profile.browser_lock["playwright"], "1.62.0")
            self.assertEqual(profile.candidate_runtime_lock["build_revision"], "6" * 40)
            self.assertEqual(
                profile.chromium, (fixture.root / "locked/chromium").resolve()
            )
            self.assertEqual(
                profile.font_bundle, (fixture.root / "locked/fonts").resolve()
            )
            self.assertEqual(
                profile.receipt_executor,
                (fixture.root / "locked/receipt-signer").resolve(),
            )
            self.assertEqual(
                profile.receipt_public_key,
                (fixture.root / "locked/public-key").resolve(),
            )
            self.assertEqual(
                profile.attestation,
                (fixture.root / "locked/attestation.json").resolve(),
            )
            self.assertEqual(profile.routing_sha256, lock["routing_table_sha256"])

    def test_schema_2_preflight_consumes_resolved_paths(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = ReceiptFixture(root / "evidence")
            self._prepare_runtime_artifacts(fixture)
            output = fixture.root / "candidate"
            project = root / "project"
            project.mkdir()
            source_set = SimpleNamespace(
                document_format=DocumentFormat.DOCX, sources=()
            )
            font = SimpleNamespace(
                manifest_sha256=sha256_file(fixture.root / "locked/fonts"),
                environment_sha256="2" * 64,
                config_path=fixture.root / "locked/configuration",
            )
            with (
                mock.patch(
                    "evaluate.multiformat_candidate_preflight.current_project_revision",
                    return_value="6" * 40,
                ),
                mock.patch(
                    "evaluate.multiformat_candidate_preflight.validate_evaluator_manifest"
                ),
                mock.patch(
                    "evaluate.multiformat_candidate_preflight.load_candidate_sources",
                    return_value=source_set,
                ),
                mock.patch(
                    "evaluate.multiformat_candidate_preflight.prepare_font_environment",
                    return_value=font,
                ),
                mock.patch(
                    "evaluate.multiformat_candidate_preflight.validate_candidate_runtime",
                    return_value={"converter_version": "test"},
                ),
                mock.patch(
                    "evaluate.multiformat_candidate_preflight.verify_candidate_attestation",
                    return_value=VerifiedAttestation("portable", "2" * 64, "e" * 64),
                ),
                mock.patch(
                    "evaluate.multiformat_candidate_preflight.importlib.metadata.version",
                    return_value="1.62.0",
                ),
            ):
                result = preflight_candidate_capture(
                    project,
                    fixture.root / "locked/contract",
                    fixture.root / "corpus/manifest.json",
                    fixture.root / "locked/evaluator",
                    fixture.lock,
                    fixture.root,
                    output,
                    converter=fixture.root / "locked/soffice",
                    soffice=fixture.root / "locked/soffice",
                    pdftohtml=fixture.root / "locked/pdftotext",
                    pdfinfo=fixture.root / "locked/pdfinfo",
                    chromium=fixture.root / "locked/chromium",
                    font_bundle=fixture.root / "locked/fonts",
                    sandbox_attestation=fixture.root / "locked/attestation.json",
                    sandbox_public_key=fixture.root / "locked/public-key",
                    openssl=fixture.root / "locked/soffice",
                    receipt_signer=fixture.root / "locked/receipt-signer",
                    timeout_seconds=30,
                    require_clean_worktree=False,
                    require_release_binary=False,
                )
            self.assertTrue(result.runtime_profile.portable)

    def test_schema_2_rejects_scope_substitution(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            fixture = ReceiptFixture(Path(temp_dir))
            self._prepare_runtime_artifacts(fixture)
            substitute = fixture.root / "substitute.json"
            substitute.write_text("{}", encoding="utf-8")

            with self.assertRaisesRegex(CandidateRuntimeProfileError, "scope"):
                resolve_candidate_runtime_profile(
                    fixture.lock,
                    fixture.root,
                    substitute,
                    fixture.root / "corpus/manifest.json",
                    fixture.root / "locked/evaluator",
                    "6" * 40,
                )

    @staticmethod
    def _prepare_runtime_artifacts(
        fixture: ReceiptFixture,
    ) -> dict[str, JsonValue]:
        lock = json.loads(fixture.lock.read_text(encoding="utf-8"))
        lock["browser"]["chromium"]["version"] = "test-chromium"
        browser_path = fixture.root / lock["browser"]["lock"]["path"]
        browser_path.write_text(
            json.dumps(
                {
                    "schema_version": 1,
                    "chromium": "test-chromium",
                    "executable_sha256": sha256_file(fixture.root / "locked/chromium"),
                    "playwright": "1.62.0",
                    "viewport_width": 1920,
                    "viewport_height": 2400,
                    "device_scale_factor": 1,
                    "locale": "en-US",
                    "timezone": "UTC",
                    "color_profile": "srgb",
                    "reduced_motion": "reduce",
                    "animations": "disabled",
                    "os": platform.system(),
                    "architecture": platform.machine(),
                    "font_environment_sha256": "2" * 64,
                },
                sort_keys=True,
            ),
            encoding="utf-8",
        )
        runtime_path = fixture.root / lock["candidate_runtime_lock"]["path"]
        runtime_path.write_text(
            json.dumps(
                {
                    "schema_version": 1,
                    "build_revision": "6" * 40,
                    "sandbox_verifier": {
                        "algorithm": "ed25519",
                        "verifier_id": "candidate-sandbox",
                        "public_key_sha256": sha256_file(
                            fixture.root / "locked/public-key"
                        ),
                        "openssl_sha256": sha256_file(fixture.root / "locked/soffice"),
                    },
                }
            ),
            encoding="utf-8",
        )
        font_path = fixture.root / lock["font_bundle"]["path"]
        font_path.write_text(
            json.dumps({"schema_version": 1, "fonts": []}), encoding="utf-8"
        )
        sandbox = fixture.root / "locked/sandbox-exec"
        sandbox.write_bytes(b"sandbox")
        profile = fixture.root / "locked/candidate.sb"
        profile.write_bytes(b"profile")
        lock["sandbox"] = {
            "executable": {
                "path": "locked/sandbox-exec",
                "sha256": sha256_file(sandbox),
            },
            "profile": {
                "path": "locked/candidate.sb",
                "sha256": sha256_file(profile),
            },
        }
        attestation_path = fixture.root / lock["runtime"]["attestation"]["path"]
        attestation = json.loads(attestation_path.read_text(encoding="utf-8"))
        attestation["sandbox_executable"] = lock["sandbox"]["executable"]
        attestation["sandbox_profile"] = lock["sandbox"]["profile"]
        attestation_path.write_text(
            json.dumps(attestation, sort_keys=True),
            encoding="utf-8",
        )
        lock["runtime"]["attestation"]["sha256"] = sha256_file(attestation_path)
        lock["browser"]["lock"]["sha256"] = sha256_file(browser_path)
        lock["candidate_runtime_lock"]["sha256"] = sha256_file(runtime_path)
        lock["font_bundle"]["sha256"] = sha256_file(font_path)
        fixture.lock.write_text(json.dumps(lock, sort_keys=True), encoding="utf-8")
        return lock


if __name__ == "__main__":
    unittest.main()
