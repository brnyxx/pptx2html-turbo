from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from evaluate.multiformat_candidate_attestation import (
    verify_candidate_attestation,
)
from evaluate.multiformat_candidate_runtime_profile import CandidateRuntimeProfile
from evaluate.multiformat_candidate_types import CandidateCaptureError
from evaluate.multiformat_reference_profile import ReferenceProfile
from evaluate.multiformat_schema import JsonValue, object_value, sha256_file
from evaluate.tests.multiformat_attestation_fixture import (
    create_test_verifier,
    verifier_lock,
    write_signed_attestation,
)


class CandidateSchema2AttestationTests(unittest.TestCase):
    def test_distinct_candidate_key_verifies_and_attacks_fail(self) -> None:
        for attack in (
            "none",
            "wrong-key",
            "signature",
            "payload",
            "transplant",
            "sandbox-path",
            "post-sign-sandbox-mutation",
            "outer-reuse",
            "extra-field",
        ):
            with self.subTest(attack=attack), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                candidate = create_test_verifier(root, name="candidate")
                wrong = create_test_verifier(root, name="wrong")
                verifier = {
                    **verifier_lock(candidate, verifier_id="candidate-sandbox"),
                    "openssl_sha256": sha256_file(candidate.openssl),
                }
                sandbox = root / "sandbox-exec"
                sandbox.write_bytes(b"sandbox")
                sandbox_profile = root / "profile.sb"
                sandbox_profile.write_bytes(b"profile")
                oracle_root = root / "reference"
                oracle_root.mkdir()
                sentinel = oracle_root / ".candidate-denial-sentinel"
                sentinel.write_bytes(b"golden")
                profile = self._profile(
                    verifier,
                    wrong.public_key,
                    root,
                    sandbox,
                    sandbox_profile,
                )
                self.assertNotEqual(profile.receipt_public_key, candidate.public_key)
                attestation = root / "attestation.json"
                payload: dict[str, JsonValue] = {
                    "schema_version": 3,
                    "status": "PASS",
                    "network_isolation": True,
                    "golden_access": "denied",
                    "sandbox_executable": {
                        "path": sandbox.name,
                        "sha256": sha256_file(sandbox),
                    },
                    "sandbox_profile": {
                        "path": sandbox_profile.name,
                        "sha256": sha256_file(sandbox_profile),
                    },
                    "network_probe": {
                        "endpoint": "1.1.1.1:443",
                        "control": "reachable",
                        "sandbox": "denied",
                    },
                    "oracle_probe": {
                        "root": {"path": oracle_root.name},
                        "sentinel": {
                            "path": sentinel.relative_to(root).as_posix(),
                            "sha256": sha256_file(sentinel),
                        },
                        "result": "denied",
                    },
                    "project_revision": "a" * 40,
                    "font_environment_sha256": "b" * 64,
                    "font_isolation": "locked-bundle-only",
                    "run_nonce": "c" * 64,
                    "verifier_id": "candidate-sandbox",
                    "scope_sha256": "d" * 64,
                }
                if attack == "transplant":
                    payload["scope_sha256"] = "e" * 64
                elif attack == "sandbox-path":
                    substitute = root / "substitute-sandbox"
                    substitute.write_bytes(sandbox.read_bytes())
                    object_value(payload, "sandbox_executable")["path"] = (
                        substitute.name
                    )
                elif attack == "extra-field":
                    payload["unrecognized"] = "signed-but-not-allowed"
                write_signed_attestation(attestation, candidate, payload)
                key = candidate.public_key
                if attack == "wrong-key":
                    key = wrong.public_key
                elif attack in {"signature", "payload"}:
                    value = json.loads(attestation.read_text(encoding="utf-8"))
                    if attack == "signature":
                        value["signature"] = "AAAA"
                    else:
                        value["golden_access"] = "allowed"
                    attestation.write_text(json.dumps(value), encoding="utf-8")
                elif attack == "post-sign-sandbox-mutation":
                    sandbox.write_bytes(b"post-sign mutation")
                elif attack == "outer-reuse":
                    attestation.write_text(
                        json.dumps(
                            {
                                "schema_version": 1,
                                "network_isolation": True,
                                "os": "Darwin",
                                "architecture": "arm64",
                            }
                        ),
                        encoding="utf-8",
                    )
                if attack == "none":
                    verified = verify_candidate_attestation(
                        profile,
                        attestation,
                        key,
                        candidate.openssl,
                        root / "lock",
                        project_revision="a" * 40,
                        scope_sha256="d" * 64,
                    )
                    self.assertEqual(verified.verifier_id, "candidate-sandbox")
                else:
                    with self.assertRaises(CandidateCaptureError):
                        verify_candidate_attestation(
                            profile,
                            attestation,
                            key,
                            candidate.openssl,
                            root / "lock",
                            project_revision="a" * 40,
                            scope_sha256="d" * 64,
                        )

    @staticmethod
    def _profile(
        verifier: dict[str, JsonValue],
        receipt_public_key: Path,
        evidence_root: Path,
        sandbox_executable: Path,
        sandbox_profile: Path,
    ) -> CandidateRuntimeProfile:
        return CandidateRuntimeProfile(
            schema_version=2,
            profile=ReferenceProfile.LIBREOFFICE_POPPLER,
            browser_lock={"font_environment_sha256": "b" * 64},
            candidate_runtime_lock={},
            sandbox_verifier=verifier,
            font_bundle=None,
            chromium=None,
            receipt_executor=None,
            receipt_public_key=receipt_public_key,
            attestation=None,
            browser_version="test",
            routing_sha256="e" * 64,
            project_revision="a" * 40,
            signer_id="outer-portable-receipt-signer",
            evidence_root=evidence_root,
            sandbox_executable=sandbox_executable,
            sandbox_profile=sandbox_profile,
            libreoffice=sandbox_executable,
        )


if __name__ == "__main__":
    unittest.main()
