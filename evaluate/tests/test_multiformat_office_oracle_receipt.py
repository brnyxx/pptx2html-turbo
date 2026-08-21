from __future__ import annotations

import json
import shutil
import tempfile
import unittest
from pathlib import Path
from typing import TypedDict

from evaluate.multiformat_evidence import oracle_lock_ready
from evaluate.multiformat_office_oracle_receipt import (
    OfficeOracleReceiptError,
    validate_office_oracle_receipt,
    write_office_oracle_receipt,
)
from evaluate.multiformat_schema import sha256_file
from evaluate.tests.multiformat_attestation_fixture import (
    create_test_verifier,
    verifier_lock,
    write_receipt_signer,
)


class OfficeReceiptFixture(TypedDict):
    evidence_root: Path
    output_dir: Path
    receipt_signer: Path
    public_key: Path
    openssl: Path
    oracle_lock: Path
    run_nonce: str
    project_revision: str
    contract_sha256: str
    corpus_sha256: str
    evaluator_sha256: str
    oracle_lock_sha256: str
    batch_manifest: Path
    runtime_identity: Path
    execution_log: Path
    artifacts: list[Path]


class MultiFormatOfficeOracleReceiptTests(unittest.TestCase):
    def test_receipt_binds_scope_runtime_log_batch_and_every_artifact(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = self._fixture(root)

            receipt = write_office_oracle_receipt(**fixture)

            validate_office_oracle_receipt(
                receipt=receipt,
                public_key=fixture["public_key"],
                openssl=fixture["openssl"],
                oracle_lock=fixture["oracle_lock"],
                run_nonce=fixture["run_nonce"],
                project_revision=fixture["project_revision"],
                contract_sha256=fixture["contract_sha256"],
                corpus_sha256=fixture["corpus_sha256"],
                evaluator_sha256=fixture["evaluator_sha256"],
                oracle_lock_sha256=fixture["oracle_lock_sha256"],
                batch_manifest=fixture["batch_manifest"],
                runtime_identity=fixture["runtime_identity"],
                execution_log=fixture["execution_log"],
                artifacts=fixture["artifacts"],
                evidence_root=root,
            )

    def test_mutated_artifact_is_rejected_after_signing(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = self._fixture(root)
            receipt = write_office_oracle_receipt(**fixture)
            fixture["artifacts"][0].write_bytes(b"mutated")

            with self.assertRaisesRegex(
                OfficeOracleReceiptError,
                "artifact",
            ):
                validate_office_oracle_receipt(
                    receipt=receipt,
                    public_key=fixture["public_key"],
                    openssl=fixture["openssl"],
                    oracle_lock=fixture["oracle_lock"],
                    run_nonce=fixture["run_nonce"],
                    project_revision=fixture["project_revision"],
                    contract_sha256=fixture["contract_sha256"],
                    corpus_sha256=fixture["corpus_sha256"],
                    evaluator_sha256=fixture["evaluator_sha256"],
                    oracle_lock_sha256=fixture["oracle_lock_sha256"],
                    batch_manifest=fixture["batch_manifest"],
                    runtime_identity=fixture["runtime_identity"],
                    execution_log=fixture["execution_log"],
                    artifacts=fixture["artifacts"],
                    evidence_root=root,
                )

    def test_oracle_lock_requires_distinct_office_verifier(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = self._fixture(root)
            lock = json.loads(fixture["oracle_lock"].read_text(encoding="utf-8"))
            del lock["office_oracle_verifier"]
            fixture["oracle_lock"].write_text(
                json.dumps(lock, sort_keys=True),
                encoding="utf-8",
            )

            self.assertFalse(oracle_lock_ready(fixture["oracle_lock"]))

    def test_oracle_and_candidate_verifiers_must_use_distinct_keys(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = self._fixture(root)
            lock = json.loads(fixture["oracle_lock"].read_text(encoding="utf-8"))
            lock["office_oracle_verifier"] = lock["sandbox_verifier"]
            fixture["oracle_lock"].write_text(
                json.dumps(lock, sort_keys=True),
                encoding="utf-8",
            )

            self.assertFalse(oracle_lock_ready(fixture["oracle_lock"]))

    def _fixture(self, root: Path) -> OfficeReceiptFixture:
        verifier = create_test_verifier(root, name="office-oracle")
        sandbox_verifier = create_test_verifier(root)
        signer = write_receipt_signer(root, verifier, name="office-oracle")
        openssl = root / "openssl"
        shutil.copy2(verifier.openssl, openssl)
        lock = root / "oracle-lock.json"
        lock.write_text(
            json.dumps(
                {
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
                        **verifier_lock(sandbox_verifier),
                        "openssl_sha256": sha256_file(openssl),
                    },
                    "office_oracle_verifier": {
                        **verifier_lock(
                            verifier,
                            verifier_id="test-office-oracle",
                        ),
                        "openssl_sha256": sha256_file(openssl),
                    },
                    "font_bundle_sha256": "5" * 64,
                },
                sort_keys=True,
            ),
            encoding="utf-8",
        )
        batch = root / "batch.json"
        runtime = root / "runtime.json"
        execution = root / "execution.json"
        artifact = root / "slide.png"
        for path, value in [
            (batch, b"batch"),
            (runtime, b"runtime"),
            (execution, b"execution"),
            (artifact, b"png"),
        ]:
            path.write_bytes(value)
        return {
            "evidence_root": root,
            "output_dir": root,
            "receipt_signer": signer,
            "public_key": verifier.public_key,
            "openssl": openssl,
            "oracle_lock": lock,
            "run_nonce": "6" * 64,
            "project_revision": "7" * 40,
            "contract_sha256": "8" * 64,
            "corpus_sha256": "9" * 64,
            "evaluator_sha256": "a" * 64,
            "oracle_lock_sha256": sha256_file(lock),
            "batch_manifest": batch,
            "runtime_identity": runtime,
            "execution_log": execution,
            "artifacts": [artifact],
        }
