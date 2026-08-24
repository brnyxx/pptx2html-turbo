from __future__ import annotations

import hashlib
import json
from pathlib import Path

from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

from evaluate.jcs import canonicalize
from evaluate.multiformat_portable_receipt import (
    PortableReceiptInput,
    PortableReceiptVerification,
    sign_portable_receipt,
    verify_portable_receipt,
)
from evaluate.multiformat_portable_receipt_trust import load_portable_receipt_trust
from evaluate.multiformat_reference_routing import load_reference_routing
from evaluate.multiformat_schema import JsonValue, sha256_file

_ROUTING = Path(__file__).resolve().parents[1] / "multiformat/reference-routing.v1.json"


class ReceiptFixtureError(TypeError):
    """The test receipt fixture has an invalid shape."""


class ReceiptFixture:
    def __init__(self, root: Path, *, nonce: str = "a" * 64) -> None:
        self.root = root
        self.private_key = Ed25519PrivateKey.generate()
        self.nonce = nonce
        self.receipt = root / "receipt.json"
        self.artifact = self._artifact("outputs/reference.pdf", b"portable reference")
        self.artifacts: list[dict[str, JsonValue]] = [self._artifact_record()]
        self.lock = self._write_lock()
        self.trust = load_portable_receipt_trust(self.lock, root)

    def verification(self, prior: tuple = ()) -> PortableReceiptVerification:
        return PortableReceiptVerification(trust=self.trust, prior_receipts=prior)

    def sign(self, output: Path | None = None) -> Path:
        return sign_portable_receipt(
            output or self.receipt,
            PortableReceiptInput(
                trust=self.trust,
                nonce=self.nonce,
                batch_id="portable-batch-1",
                artifacts=self.artifacts,
            ),
            self.private_key,
        )

    def verify(self, verification: PortableReceiptVerification | None = None):
        return verify_portable_receipt(
            self.receipt,
            verification or self.verification(),
        )

    def read_receipt(self) -> dict[str, JsonValue]:
        value = json.loads(self.receipt.read_text(encoding="utf-8"))
        if not isinstance(value, dict):
            raise ReceiptFixtureError("receipt")
        return value

    def resign(self, value: dict[str, JsonValue]) -> None:
        runtime = _mapping(value, "runtime")
        artifacts = _objects(value, "artifacts")
        payload = {"runtime": runtime, "artifacts": artifacts}
        digest = hashlib.sha256(canonicalize(payload)).digest()
        value["payload_sha256"] = digest.hex()
        value["signature"] = self.private_key.sign(digest).hex()
        self.receipt.write_bytes(canonicalize(value))

    def _write_lock(self) -> Path:
        raw_key = self.private_key.public_key().public_bytes_raw()
        artifacts = {
            name: self._artifact(f"locked/{name}", content)
            for name, content in {
                "soffice": b"soffice",
                "pdftoppm": b"pdftoppm",
                "pdftotext": b"pdftotext",
                "pdfinfo": b"pdfinfo",
                "canonicalizer": b"canonicalizer",
                "fonts": b"fonts",
                "configuration": b"configuration",
                "chromium": b"chromium",
                "candidate-runtime-lock": b"candidate-runtime-lock",
                "browser-lock": b"browser-lock",
                "public-key": raw_key,
                "executor": b"executor",
                "contract": b"contract",
                "evaluator": b"evaluator",
            }.items()
        }
        source = self._artifact("corpus/source.docx", b"source")
        corpus = self.root / "corpus/manifest.json"
        corpus.write_text(
            json.dumps(
                {
                    "sources": [
                        {
                            "path": source.name,
                            "sha256": sha256_file(source),
                        }
                    ]
                },
                sort_keys=True,
            ),
            encoding="utf-8",
        )
        attestation = self.root / "locked/attestation.json"
        attestation.write_text(
            json.dumps(
                {
                    "schema_version": 1,
                    "os": "Darwin",
                    "architecture": "arm64",
                    "locale": "en-US",
                    "timezone": "UTC",
                    "rendering_dpi": 144,
                    "network_isolation": True,
                },
                sort_keys=True,
            ),
            encoding="utf-8",
        )
        binding = {name: self._binding(path) for name, path in artifacts.items()}
        lock: dict[str, JsonValue] = {
            "schema_version": 2,
            "status": "locked",
            "reference_profile": "libreoffice-poppler",
            "platform": {"os": "Darwin", "architecture": "arm64"},
            "tools": {
                "libreoffice": {"version": "test", **binding["soffice"]},
                "poppler_render": {"version": "test", **binding["pdftoppm"]},
                "poppler_text": {"version": "test", **binding["pdftotext"]},
                "poppler_metadata": {"version": "test", **binding["pdfinfo"]},
            },
            "routing_table_sha256": load_reference_routing(_ROUTING).sha256,
            "canonicalizer": {"version": "1", **binding["canonicalizer"]},
            "font_bundle": {"version": "test", **binding["fonts"]},
            "configuration": {"version": "test", **binding["configuration"]},
            "browser": {
                "chromium": {"version": "test", **binding["chromium"]},
                "lock": binding["browser-lock"],
            },
            "candidate_runtime_lock": binding["candidate-runtime-lock"],
            "signer": {
                "algorithm": "ed25519",
                "signer_id": "multiformat-portable-reference-v1",
                "public_key": binding["public-key"],
                "receipt_schema_version": 1,
                "executor": binding["executor"],
            },
            "scope": {
                "contract": binding["contract"],
                "evaluator": binding["evaluator"],
                "corpus": self._binding(corpus),
                "project_revision": "6" * 40,
            },
            "runtime": {
                "locale": "en-US",
                "timezone": "UTC",
                "rendering_dpi": 144,
                "network_isolation": True,
                "attestation": self._binding(attestation),
            },
        }
        path = self.root / "portable-lock.json"
        path.write_text(json.dumps(lock, sort_keys=True), encoding="utf-8")
        return path

    def _artifact_record(
        self, *, path: Path | None = None, role: str = "canonical-pdf"
    ) -> dict[str, JsonValue]:
        artifact = path or self.artifact
        return {
            "path": artifact.relative_to(self.root).as_posix(),
            "sha256": sha256_file(artifact),
            "size": artifact.stat().st_size,
            "role": role,
        }

    def _artifact(self, relative: str, content: bytes) -> Path:
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(content)
        return path

    def _binding(self, path: Path) -> dict[str, JsonValue]:
        return {
            "path": path.relative_to(self.root).as_posix(),
            "sha256": sha256_file(path),
        }


def _mapping(value: dict[str, JsonValue], field: str) -> dict[str, JsonValue]:
    result = value[field]
    if not isinstance(result, dict):
        raise ReceiptFixtureError(field)
    return result


def _objects(value: dict[str, JsonValue], field: str) -> list[dict[str, JsonValue]]:
    result = value[field]
    if not isinstance(result, list) or not all(
        isinstance(item, dict) for item in result
    ):
        raise ReceiptFixtureError(field)
    return result
