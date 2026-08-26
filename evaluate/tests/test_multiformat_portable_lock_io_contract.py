from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from evaluate.multiformat_portable_lock_io import validate_candidate_locks
from evaluate.multiformat_portable_package_inventory import PortableLockIoError


class PortableLockIoContractTests(unittest.TestCase):
    def test_fields_only_browser_lock_is_accepted(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            # Given: the fields-only browser contract and a schema-1 runtime input.
            browser, runtime = self._locks(Path(temporary))

            # When/Then: both materializer inputs validate without a browser schema tag.
            validate_candidate_locks(browser, runtime)

    def test_browser_schema_version_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            # Given: a browser lock carrying the contradictory schema_version field.
            browser, runtime = self._locks(Path(temporary))
            value = json.loads(browser.read_text(encoding="utf-8"))
            value["schema_version"] = 1
            browser.write_text(json.dumps(value, sort_keys=True), encoding="utf-8")

            # When/Then: exact fields-only browser validation rejects it.
            with self.assertRaisesRegex(
                PortableLockIoError, "browser lock is incomplete"
            ):
                validate_candidate_locks(browser, runtime)

    @staticmethod
    def _locks(root: Path) -> tuple[Path, Path]:
        browser = root / "browser.json"
        browser.write_text(
            json.dumps(
                {
                    "chromium": "test",
                    "executable_sha256": "0" * 64,
                    "playwright": "test",
                    "os": "Linux",
                    "architecture": "x86_64",
                    "font_environment_sha256": "1" * 64,
                    "viewport_width": 1920,
                    "viewport_height": 2400,
                    "device_scale_factor": 1,
                    "locale": "en-US",
                    "timezone": "UTC",
                    "color_profile": "srgb",
                    "reduced_motion": "reduce",
                    "animations": "disabled",
                },
                sort_keys=True,
            ),
            encoding="utf-8",
        )
        runtime = root / "runtime.json"
        runtime.write_text(
            json.dumps(
                {
                    "schema_version": 1,
                    "status": "locked",
                    "browser": {},
                    "candidate_runtime": {
                        "build_revision": "a" * 40,
                        "converter_sha256": "0" * 64,
                        "converter_version": "test",
                        "soffice_sha256": "0" * 64,
                        "soffice_version": "test",
                        "pdftohtml_sha256": "0" * 64,
                        "pdftohtml_version": "test",
                        "pdfinfo_sha256": "0" * 64,
                        "pdfinfo_version": "test",
                        "receipt_signer_sha256": "0" * 64,
                        "receipt_signer_version": "test",
                    },
                    "sandbox_verifier": {
                        "algorithm": "ed25519",
                        "verifier_id": "test",
                        "public_key_sha256": "0" * 64,
                        "openssl_sha256": "0" * 64,
                    },
                    "font_bundle_sha256": "0" * 64,
                },
                sort_keys=True,
            ),
            encoding="utf-8",
        )
        return browser, runtime


if __name__ == "__main__":
    unittest.main()
