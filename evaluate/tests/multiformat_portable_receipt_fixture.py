from __future__ import annotations

import hashlib
import json
from pathlib import Path
from typing import cast

from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

from evaluate.jcs import canonicalize
from evaluate.multiformat_portable_receipt import (
    PortableReceiptInput,
    PortableReceiptVerification,
    sign_portable_receipt,
    verify_portable_receipt,
)
from evaluate.multiformat_portable_receipt_nonce import (
    PortableReceiptClaim,
    artifact_root_sha256,
    canonical_receipt_path,
    portable_receipt_nonce,
)
from evaluate.multiformat_portable_receipt_trust import load_portable_receipt_trust
from evaluate.multiformat_schema import JsonValue, sha256_file
from evaluate.tests.multiformat_outer_lock_fixture import (
    write_portable_receipt_lock,
)


class ReceiptFixtureError(TypeError):
    """The test receipt fixture has an invalid shape."""


class ReceiptFixture:
    def __init__(
        self,
        root: Path,
        *,
        candidate_runtime_lock: Path | None = None,
    ) -> None:
        self.root = root
        self.private_key = Ed25519PrivateKey.generate()
        self.candidate_runtime_lock = candidate_runtime_lock
        self.receipt = root / "receipt.json"
        self.artifact = self._artifact("outputs/reference.pdf", b"portable reference")
        self.artifacts: list[dict[str, JsonValue]] = [self._artifact_record()]
        self.lock = self._write_lock()
        self.trust = load_portable_receipt_trust(self.lock, root)
        self.nonce = self.expected_nonce()

    def verification(
        self,
        *,
        bound_receipt_path: Path | None = None,
    ) -> PortableReceiptVerification:
        return PortableReceiptVerification(
            trust=self.trust,
            bound_receipt_path=bound_receipt_path,
        )

    def sign(
        self,
        output: Path | None = None,
        *,
        batch_id: str = "portable-batch-1",
    ) -> Path:
        destination = output or self.receipt
        self.nonce = self.expected_nonce(batch_id=batch_id, output=destination)
        return sign_portable_receipt(
            destination,
            PortableReceiptInput(
                trust=self.trust,
                batch_id=batch_id,
                artifacts=self.artifacts,
            ),
            self.private_key,
        )

    def verify(self, verification: PortableReceiptVerification | None = None):
        return verify_portable_receipt(
            self.receipt,
            verification or self.verification(),
        )

    def expected_nonce(
        self,
        *,
        batch_id: str = "portable-batch-1",
        output: Path | None = None,
    ) -> str:
        return portable_receipt_nonce(
            PortableReceiptClaim(
                scope_sha256=self.trust.scope_sha256,
                batch_id=batch_id,
                artifact_root_sha256=artifact_root_sha256(self.artifacts),
                receipt_path=canonical_receipt_path(
                    self.root,
                    output or self.receipt,
                ),
            )
        )

    def read_receipt(self) -> dict[str, JsonValue]:
        value = json.loads(self.receipt.read_text(encoding="utf-8"))
        if not isinstance(value, dict):
            raise ReceiptFixtureError("receipt")
        return value

    def resign(self, value: dict[str, JsonValue]) -> None:
        runtime = _mapping(value, "runtime")
        artifacts = _objects(value, "artifacts")
        batch_id = runtime.get("batch_id")
        if not isinstance(batch_id, str):
            raise ReceiptFixtureError("runtime.batch_id")
        runtime["nonce"] = portable_receipt_nonce(
            PortableReceiptClaim(
                scope_sha256=self.trust.scope_sha256,
                batch_id=batch_id,
                artifact_root_sha256=artifact_root_sha256(artifacts),
                receipt_path=canonical_receipt_path(self.root, self.receipt),
            )
        )
        payload: JsonValue = {
            "runtime": runtime,
            "artifacts": cast(JsonValue, artifacts),
        }
        digest = hashlib.sha256(canonicalize(payload)).digest()
        value["payload_sha256"] = digest.hex()
        value["signature"] = self.private_key.sign(digest).hex()
        self.receipt.write_bytes(canonicalize(value))

    def _write_lock(self) -> Path:
        return write_portable_receipt_lock(
            self.root,
            self.private_key.public_key().public_bytes_raw(),
            self.candidate_runtime_lock,
        )

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
    if not isinstance(result, list):
        raise ReceiptFixtureError(field)
    objects: list[dict[str, JsonValue]] = []
    for item in result:
        if not isinstance(item, dict):
            raise ReceiptFixtureError(field)
        objects.append(item)
    return objects
