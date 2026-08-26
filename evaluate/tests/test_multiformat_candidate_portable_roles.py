from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from evaluate.multiformat_candidate_portable_receipt import (
    CandidatePortableReceiptError,
    validate_candidate_capture_roles,
)
from evaluate.multiformat_candidate_receipt import write_execution_receipt
from evaluate.multiformat_candidate_runtime_profile import CandidateRuntimeProfile
from evaluate.multiformat_candidate_types import (
    CandidateRun,
    CapturedSource,
    CapturedUnit,
)
from evaluate.multiformat_capture_provenance import validate_portable_capture_provenance
from evaluate.multiformat_portable_receipt import (
    PortableReceiptInput,
    sign_portable_receipt,
)
from evaluate.multiformat_reference_profile import ReferenceProfile
from evaluate.tests.multiformat_portable_receipt_fixture import ReceiptFixture


class CandidatePortableRoleTests(unittest.TestCase):
    def test_writer_validates_and_role_tamper_fails_profile_semantics(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            fixture = ReceiptFixture(Path(temp_dir))
            output = fixture.root / "capture"
            output.mkdir()
            runtime = self._file(output / "runtime.json", "runtime")
            execution = self._file(output / "execution.json", "execution")
            determinism = self._file(output / "determinism.json", "determinism")

            def execute(_executor: Path, request: Path, receipt: Path) -> None:
                value = json.loads(request.read_text(encoding="utf-8"))
                sign_portable_receipt(
                    receipt,
                    PortableReceiptInput(
                        trust=fixture.trust,
                        batch_id=value["batch_id"],
                        artifacts=value["artifacts"],
                    ),
                    fixture.private_key,
                )

            receipt = write_execution_receipt(
                fixture.root,
                output,
                fixture.root / "locked/executor",
                fixture.root / "locked/public-key",
                fixture.root / "locked/soffice",
                fixture.lock,
                run_nonce="f" * 64,
                project_revision="6" * 40,
                contract_sha256="1" * 64,
                corpus_sha256="2" * 64,
                evaluator_sha256="3" * 64,
                oracle_lock_sha256="4" * 64,
                runtime_identity=runtime,
                execution_log=execution,
                runtime_profile=self._profile(),
                determinism=determinism,
                runs=(self._run(output, 1), self._run(output, 2)),
                runtime_artifacts={},
                portable_execute=execute,
            )
            verified = validate_portable_capture_provenance(
                receipt, fixture.verification()
            )
            validate_candidate_capture_roles(verified)
            roles = {artifact.role for artifact in verified.artifacts}
            self.assertTrue(
                {
                    "capture-runtime-identity",
                    "capture-execution-log",
                    "capture-unit-png",
                    "capture-unit-inventory",
                    "capture-candidate-html",
                    "capture-candidate-determinism",
                }.issubset(roles)
            )

            value = json.loads(receipt.read_text(encoding="utf-8"))
            for item in value["artifacts"]:
                if item["role"] == "capture-unit-png":
                    item["role"] = "candidate-png"
            fixture.receipt = receipt
            fixture.resign(value)
            tampered = validate_portable_capture_provenance(
                receipt, fixture.verification()
            )
            with self.assertRaises(CandidatePortableReceiptError):
                validate_candidate_capture_roles(tampered)

    @staticmethod
    def _file(path: Path, value: str) -> Path:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(value, encoding="utf-8")
        return path

    @classmethod
    def _run(cls, root: Path, run_id: int) -> CandidateRun:
        base = root / f"run-{run_id}"
        html = cls._file(base / "document.html", "<html></html>")
        manifest = cls._file(base / "inventory-manifest.json", "{}")
        png = cls._file(base / "unit.png", "png")
        inventory = cls._file(base / "unit.json", "{}")
        unit = CapturedUnit("unit-1", png, inventory)
        source = CapturedSource(
            "conformance", "source-1", "a" * 64, html, manifest, (unit,)
        )
        return CandidateRun(run_id, "test", (source,))

    @staticmethod
    def _profile() -> CandidateRuntimeProfile:
        return CandidateRuntimeProfile(
            schema_version=2,
            profile=ReferenceProfile.LIBREOFFICE_POPPLER,
            browser_lock={},
            candidate_runtime_lock={},
            sandbox_verifier={},
            font_bundle=None,
            chromium=None,
            receipt_executor=None,
            receipt_public_key=None,
            attestation=None,
            browser_version="test",
            routing_sha256=None,
            project_revision="6" * 40,
            signer_id="portable-signer",
        )


if __name__ == "__main__":
    unittest.main()
