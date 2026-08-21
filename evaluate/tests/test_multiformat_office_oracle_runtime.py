from __future__ import annotations

import json
import shutil
import tempfile
import unittest
from pathlib import Path

from evaluate.multiformat_metric_types import MetricError
from evaluate.multiformat_office_oracle_runtime import (
    validate_office_oracle_runtime,
)
from evaluate.multiformat_schema import sha256_file
from evaluate.tests.multiformat_attestation_fixture import (
    create_test_verifier,
    verifier_lock,
    write_receipt_signer,
)
from evaluate.tests.multiformat_metric_artifact_fixture import binding


class MultiFormatOfficeOracleRuntimeTests(unittest.TestCase):
    def test_runtime_binds_office_versions_and_verifier_binaries(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            runtime, lock = self._fixture(root)

            paths = validate_office_oracle_runtime(
                runtime,
                lock,
                root,
                "windows-office-native",
            )

            self.assertEqual(
                set(paths),
                {
                    "office_oracle_public_key",
                    "openssl_binary",
                    "receipt_signer_binary",
                },
            )

    def test_runtime_version_drift_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            runtime, lock = self._fixture(root)
            values = json.loads(runtime.read_text(encoding="utf-8"))
            values["tools"]["word_version"] = "17.0"
            runtime.write_text(json.dumps(values, sort_keys=True), encoding="utf-8")

            with self.assertRaisesRegex(MetricError, "word"):
                validate_office_oracle_runtime(
                    runtime,
                    lock,
                    root,
                    "windows-office-native",
                )

    def _fixture(self, root: Path) -> tuple[Path, Path]:
        verifier = create_test_verifier(root, name="office-oracle")
        signer = write_receipt_signer(
            root,
            verifier,
            name="office-oracle",
        )
        openssl = root / "openssl"
        shutil.copy2(verifier.openssl, openssl)
        lock = root / "oracle-lock.json"
        lock.write_text(
            json.dumps(
                {
                    "office": {
                        "os": "Windows 11",
                        "channel": "test",
                        "word": "16.0",
                        "excel": "16.0",
                        "powerpoint": "16.0",
                    },
                    "pdf": {
                        "primary": "pdfinfo 25",
                        "secondary": "pdftoppm 25",
                        "text": "pdftotext 25",
                    },
                    "office_oracle_verifier": {
                        **verifier_lock(
                            verifier,
                            verifier_id="test-office-oracle",
                        ),
                        "openssl_sha256": sha256_file(openssl),
                    },
                },
                sort_keys=True,
            ),
            encoding="utf-8",
        )
        runtime = root / "runtime.json"
        runtime.write_text(
            json.dumps(
                {
                    "os": "Windows 11",
                    "tools": {
                        "word_version": "16.0",
                        "office_channel": "test",
                        "excel_version": "16.0",
                        "powerpoint_version": "16.0",
                        "pdf_primary_version": "pdfinfo 25",
                        "pdf_secondary_version": "pdftoppm 25",
                        "pdf_text_version": "pdftotext 25",
                        "office_oracle_public_key_sha256": sha256_file(
                            verifier.public_key
                        ),
                        "openssl_sha256": sha256_file(openssl),
                        "receipt_signer_sha256": sha256_file(signer),
                        "receipt_signer_version": "test",
                        "office_oracle_verifier_id": "test-office-oracle",
                    },
                    "artifacts": {
                        "office_oracle_public_key": binding(
                            root,
                            verifier.public_key,
                        ),
                        "openssl_binary": binding(root, openssl),
                        "receipt_signer_binary": binding(root, signer),
                    },
                },
                sort_keys=True,
            ),
            encoding="utf-8",
        )
        return runtime, lock
