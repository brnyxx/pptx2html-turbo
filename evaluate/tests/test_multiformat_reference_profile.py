from __future__ import annotations

import hashlib
import json
import tempfile
import unittest
from pathlib import Path

from evaluate.multiformat_reference_profile import (
    ReferenceLockIdentity,
    ReferenceProfile,
    ReferenceProfileError,
    load_reference_lock_identity,
)
from evaluate.multiformat_schema import JsonValue


class MultiFormatReferenceProfileTests(unittest.TestCase):
    def test_profiles_have_stable_wire_values(self) -> None:
        self.assertEqual(
            ReferenceProfile.LIBREOFFICE_POPPLER.value,
            "libreoffice-poppler",
        )
        self.assertEqual(
            ReferenceProfile.MICROSOFT_OFFICE.value,
            "microsoft-office",
        )

    def test_explicit_schema_2_profiles_are_parsed_and_hashed(self) -> None:
        for profile in ReferenceProfile:
            with (
                self.subTest(profile=profile),
                tempfile.TemporaryDirectory() as temp_dir,
            ):
                path = Path(temp_dir) / "lock.json"
                path.write_text(
                    json.dumps(
                        {
                            "schema_version": 2,
                            "reference_profile": profile.value,
                        },
                        sort_keys=True,
                    ),
                    encoding="utf-8",
                )
                lock_bytes = path.read_bytes()

                identity = load_reference_lock_identity(path)

                self.assertEqual(
                    identity,
                    ReferenceLockIdentity(
                        schema_version=2,
                        profile=profile,
                        sha256=hashlib.sha256(lock_bytes).hexdigest(),
                    ),
                )

    def test_schema_1_office_lock_adapts_without_rewriting_bytes(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            path = Path(temp_dir) / "office-lock.json"
            path.write_text(
                json.dumps(self._legacy_office_lock(), sort_keys=True),
                encoding="utf-8",
            )
            original_bytes = path.read_bytes()

            identity = load_reference_lock_identity(path)

            self.assertEqual(identity.schema_version, 1)
            self.assertIs(identity.profile, ReferenceProfile.MICROSOFT_OFFICE)
            self.assertEqual(
                identity.sha256,
                hashlib.sha256(original_bytes).hexdigest(),
            )
            self.assertEqual(path.read_bytes(), original_bytes)

    def test_unknown_profile_or_schema_combination_fails_closed(self) -> None:
        cases = [
            {"schema_version": 2},
            {"schema_version": 2, "reference_profile": "unknown"},
            {
                "schema_version": 1,
                "reference_profile": ReferenceProfile.MICROSOFT_OFFICE.value,
            },
            {
                "schema_version": 3,
                "reference_profile": ReferenceProfile.LIBREOFFICE_POPPLER.value,
            },
        ]
        for values in cases:
            with self.subTest(values=values), tempfile.TemporaryDirectory() as temp_dir:
                path = Path(temp_dir) / "invalid-lock.json"
                path.write_text(json.dumps(values), encoding="utf-8")

                with self.assertRaises(ReferenceProfileError):
                    load_reference_lock_identity(path)

    def test_malformed_legacy_hash_fails_closed(self) -> None:
        lock = self._legacy_office_lock()
        lock["font_bundle_sha256"] = "not-a-sha256"
        with tempfile.TemporaryDirectory() as temp_dir:
            path = Path(temp_dir) / "invalid-office-lock.json"
            path.write_text(json.dumps(lock, sort_keys=True), encoding="utf-8")

            with self.assertRaises(ReferenceProfileError):
                load_reference_lock_identity(path)

    @staticmethod
    def _legacy_office_lock() -> dict[str, JsonValue]:
        return {
            "schema_version": 1,
            "status": "locked",
            "office": {
                "os": "Windows 11",
                "channel": "test",
                "word": "16.0",
                "excel": "16.0",
                "powerpoint": "16.0",
            },
            "pdf": {
                "primary": "pdfinfo",
                "secondary": "pdftoppm",
                "text": "pdftotext",
            },
            "browser": {
                "chromium": "test",
                "executable_sha256": "1" * 64,
                "playwright": "1.62.0",
                "viewport_width": 1920,
                "viewport_height": 2400,
                "device_scale_factor": 1,
                "locale": "en-US",
                "timezone": "UTC",
                "color_profile": "srgb",
                "reduced_motion": "reduce",
                "animations": "disabled",
                "os": "test",
                "architecture": "x86_64",
                "font_environment_sha256": "2" * 64,
            },
            "candidate_runtime": {
                "build_revision": "3" * 40,
                **{
                    f"{name}_sha256": "4" * 64
                    for name in [
                        "converter",
                        "soffice",
                        "pdftohtml",
                        "pdfinfo",
                        "receipt_signer",
                    ]
                },
                **{
                    f"{name}_version": "test"
                    for name in [
                        "converter",
                        "soffice",
                        "pdftohtml",
                        "pdfinfo",
                        "receipt_signer",
                    ]
                },
            },
            "sandbox_verifier": {
                "algorithm": "ed25519",
                "verifier_id": "sandbox",
                "public_key_sha256": "5" * 64,
                "openssl_sha256": "6" * 64,
            },
            "office_oracle_verifier": {
                "algorithm": "ed25519",
                "verifier_id": "office",
                "public_key_sha256": "7" * 64,
                "openssl_sha256": "6" * 64,
            },
            "font_bundle_sha256": "8" * 64,
        }
