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
from evaluate.multiformat_schema import JsonValue, sha256_file
from evaluate.tests.multiformat_attestation_fixture import (
    create_test_verifier,
    verifier_lock,
    write_signed_attestation,
)


class CandidateSchema2AttestationTests(unittest.TestCase):
    def test_distinct_candidate_key_verifies_and_attacks_fail(self) -> None:
        for attack in ("none", "wrong-key", "signature", "payload"):
            with self.subTest(attack=attack), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                candidate = create_test_verifier(root, name="candidate")
                wrong = create_test_verifier(root, name="wrong")
                verifier = {
                    **verifier_lock(candidate, verifier_id="candidate-sandbox"),
                    "openssl_sha256": sha256_file(candidate.openssl),
                }
                profile = self._profile(verifier, wrong.public_key)
                self.assertNotEqual(profile.receipt_public_key, candidate.public_key)
                attestation = root / "attestation.json"
                payload: dict[str, JsonValue] = {
                    "schema_version": 1,
                    "status": "PASS",
                    "network_isolation": True,
                    "golden_access": "denied",
                    "project_revision": "a" * 40,
                    "font_environment_sha256": "b" * 64,
                    "font_isolation": "locked-bundle-only",
                    "run_nonce": "c" * 64,
                    "verifier_id": "candidate-sandbox",
                }
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
        verifier: dict[str, JsonValue], receipt_public_key: Path
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
        )


if __name__ == "__main__":
    unittest.main()
