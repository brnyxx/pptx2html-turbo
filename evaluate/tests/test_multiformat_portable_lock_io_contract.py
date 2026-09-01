from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from evaluate.multiformat_portable_lock_io import (
    validate_candidate_artifacts,
    validate_candidate_locks,
)
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

    def test_schema_two_missing_package_inventory_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            # Given: a schema-2 candidate runtime lock with incomplete package paths.
            runtime, paths, versions, revision = self._candidate_artifacts(
                Path(temporary)
            )

            # When/Then: missing package inventory keys fail closed with a typed error.
            for missing in (
                "poppler-package-inventory",
                "openssl-package-inventory",
            ):
                with self.subTest(missing=missing):
                    missing_paths = dict(paths)
                    missing_paths.pop(missing)
                    with (
                        self.assertRaisesRegex(
                            PortableLockIoError,
                            "portable candidate package inventory is missing",
                        ),
                        patch(
                            "evaluate.multiformat_portable_lock_io.sha256_file",
                            return_value="0" * 64,
                        ),
                    ):
                        validate_candidate_artifacts(
                            runtime, missing_paths, versions, revision
                        )

    @staticmethod
    def _candidate_artifacts(
        root: Path,
    ) -> tuple[Path, dict[str, Path], dict[str, str], str]:
        browser, runtime = PortableLockIoContractTests._locks(root)
        runtime_value = json.loads(runtime.read_text(encoding="utf-8"))
        runtime_value["schema_version"] = 2
        runtime_value["browser"] = json.loads(browser.read_text(encoding="utf-8"))
        runtime_value["candidate_runtime"]["poppler_package_inventory_sha256"] = (
            "0" * 64
        )
        runtime_value["sandbox_verifier"]["openssl_package_inventory_sha256"] = "0" * 64
        runtime.write_text(json.dumps(runtime_value, sort_keys=True), encoding="utf-8")
        paths: dict[str, Path] = {}
        for name in (
            "browser-lock",
            "chromium",
            "converter",
            "libreoffice",
            "pdftohtml",
            "poppler-metadata",
            "receipt-signer",
            "candidate-sandbox-public-key",
            "openssl",
            "font-bundle",
            "poppler-package-inventory",
            "openssl-package-inventory",
        ):
            path = root / f"{name}.json"
            path.write_text(name, encoding="utf-8")
            paths[name] = path
        paths["browser-lock"] = browser
        versions = {
            "chromium": "test",
            "converter": "test",
            "libreoffice": "test",
            "pdftohtml": "test",
            "poppler-metadata": "test",
            "receipt-signer": "test",
        }
        return (
            runtime,
            paths,
            versions,
            runtime_value["candidate_runtime"]["build_revision"],
        )

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
