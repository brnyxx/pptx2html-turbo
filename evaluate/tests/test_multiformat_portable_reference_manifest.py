from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

from evaluate.multiformat_candidate_sources import (
    CandidateSource,
    CandidateSourceSet,
    CandidateUnitSpec,
)
from evaluate.multiformat_capture_manifest import validate_capture_manifest
from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_metric_links import load_metric_spec
from evaluate.multiformat_metric_types import MetricError
from evaluate.multiformat_office_oracle_batch import (
    OfficeOracleBatchFile,
    OfficeOracleBatchUnit,
)
from evaluate.multiformat_portable_receipt import (
    PortableReceiptVerification,
    verify_portable_receipt,
)
from evaluate.multiformat_portable_receipt_executor import execute_receipt_request
from evaluate.multiformat_portable_reference_manifest import (
    write_portable_reference_manifests,
)
from evaluate.multiformat_schema import sha256_file
from evaluate.tests.multiformat_metric_artifact_fixture import write_png
from evaluate.tests.multiformat_portable_receipt_fixture import ReceiptFixture


class SandboxReceiptFixture(ReceiptFixture):
    def _write_lock(self) -> Path:
        path = super()._write_lock()
        sandbox = self._artifact("locked/sandbox-exec", b"sandbox")
        profile = self._artifact(
            "locked/profile.sb",
            b"(version 1)\n(allow default)\n(deny network*)\n(allow network* (local unix-socket))\n(allow network* (remote unix-socket))\n",
        )
        lock = json.loads(path.read_text(encoding="utf-8"))
        corpus_path = self.root / lock["scope"]["corpus"]["path"]
        source = self.root / "corpus/source.docx"
        corpus_path.write_text(
            json.dumps(
                {
                    "format": "docx",
                    "tracks": {
                        "conformance": {
                            "items": [
                                {
                                    "id": "source",
                                    "path": "source.docx",
                                    "sha256": sha256_file(source),
                                    "units": [
                                        {
                                            "id": "unit-1",
                                            "ordinal": 1,
                                            "primary_stratum": "text",
                                            "applicable_metrics": [
                                                "visual",
                                                "content",
                                                "layout",
                                            ],
                                            "background": "#ffffff",
                                        }
                                    ],
                                }
                            ]
                        },
                        "blind": {"items": []},
                        "security": {"items": []},
                    },
                },
                sort_keys=True,
            ),
            encoding="utf-8",
        )
        lock["scope"]["corpus"] = self._binding(corpus_path)
        attestation_path = self.root / lock["runtime"]["attestation"]["path"]
        attestation = json.loads(attestation_path.read_text(encoding="utf-8"))
        attestation["sandbox_executable"] = self._binding(sandbox)
        attestation["sandbox_profile"] = self._binding(profile)
        attestation_path.write_text(
            json.dumps(attestation, sort_keys=True), encoding="utf-8"
        )
        lock["runtime"]["attestation"] = self._binding(attestation_path)
        lock["sandbox"] = {
            "executable": self._binding(sandbox),
            "profile": self._binding(profile),
        }
        path.write_text(json.dumps(lock, sort_keys=True), encoding="utf-8")
        return path


class PortableReferenceManifestTests(unittest.TestCase):
    def test_signs_verifies_and_rejects_wrong_private_key(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            base = Path(temp_dir)
            evidence = base / "evidence"
            evidence.mkdir()
            fixture = SandboxReceiptFixture(evidence)
            source_path = fixture.root / "corpus/source.docx"
            source = CandidateSource(
                "conformance",
                "source",
                sha256_file(source_path),
                source_path,
                (CandidateUnitSpec("unit-1", 1),),
            )
            sources = CandidateSourceSet(DocumentFormat.DOCX, (source,))
            raw = fixture.root / "raw"
            raw.mkdir()
            png = raw / "page-1.png"
            write_png(png, 200, 200, (1, 2, 3))
            pdf, semantic, layout = (
                raw / "reference.pdf",
                raw / "semantic.json",
                raw / "layout.xml",
            )
            pdf.write_bytes(b"pdf")
            semantic.write_text("{}")
            layout.write_text(
                '<doc><page width="200" height="200"><line><word xMin="1" yMin="1" xMax="2" yMax="2">x</word></line></page></doc>'
            )
            batch = OfficeOracleBatchFile(
                "source",
                "docx",
                source.source_sha256,
                pdf,
                semantic,
                layout,
                (OfficeOracleBatchUnit(png, 200, 200),),
            )
            published = fixture.root / "published"
            published.mkdir()
            private_key = base / "private.raw"
            private_key.write_bytes(fixture.private_key.private_bytes_raw())
            private_key.chmod(0o600)

            def execute(_executor: Path, request: Path, output: Path) -> None:
                execute_receipt_request(
                    request,
                    output,
                    fixture.lock,
                    fixture.root,
                    private_key,
                )

            capture = write_portable_reference_manifests(
                published,
                sources,
                [batch],
                fixture.trust,
                Path("unused-injected-executor"),
                batch_id="batch-1",
                execute=execute,
            )
            identity = verify_portable_receipt(
                published / "portable-receipt.json",
                PortableReceiptVerification(fixture.trust),
            )
            self.assertTrue(identity.is_verified())
            self.assertTrue(capture.is_file())
            tampered = json.loads(capture.read_text(encoding="utf-8"))
            tampered["units"][0]["png"]["sha256"] = "0" * 64
            capture.write_text(json.dumps(tampered, sort_keys=True), encoding="utf-8")
            by_role = {
                item.role: fixture.root / item.path
                for item in fixture.trust.lock_artifacts
            }
            with self.assertRaises(MetricError):
                validate_capture_manifest(
                    capture,
                    "oracle",
                    load_metric_spec(by_role["corpus-manifest"]),
                    fixture.trust.contract_sha256,
                    fixture.trust.corpus_sha256,
                    fixture.trust.evaluator_sha256,
                    fixture.trust.lock_sha256,
                    fixture.trust.project_revision,
                    fixture.root,
                    by_role["portable-lock"],
                )

            other = fixture.root / "other"
            other.mkdir()

            wrong_key = base / "wrong-private.raw"
            wrong_key.write_bytes(Ed25519PrivateKey.generate().private_bytes_raw())
            wrong_key.chmod(0o600)

            def reject(_executor: Path, request: Path, output: Path) -> None:
                execute_receipt_request(
                    request,
                    output,
                    fixture.lock,
                    fixture.root,
                    wrong_key,
                )

            with self.assertRaises(ValueError):
                write_portable_reference_manifests(
                    other,
                    sources,
                    [batch],
                    fixture.trust,
                    Path("/missing-receipt-executor"),
                    batch_id="batch-2",
                    execute=reject,
                )


if __name__ == "__main__":
    unittest.main()
