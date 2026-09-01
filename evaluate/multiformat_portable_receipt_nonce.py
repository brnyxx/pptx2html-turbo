"""Deterministic claim identities for stateless portable receipts."""

from __future__ import annotations

import hashlib
from dataclasses import dataclass
from pathlib import Path
from typing import Final, cast

from evaluate.jcs import canonicalize
from evaluate.multiformat_schema import JsonValue

_NONCE_DOMAIN: Final = "pptx2html.portable-receipt.nonce.v2"


class PortableReceiptNonceError(ValueError):
    """A receipt claim path cannot be bound canonically."""


@dataclass(frozen=True, slots=True)
class PortableReceiptClaim:
    scope_sha256: str
    batch_id: str
    artifact_root_sha256: str
    receipt_path: str


def artifact_root_sha256(artifacts: list[dict[str, JsonValue]]) -> str:
    """Return the canonical root of one complete signed artifact set."""
    value = cast(JsonValue, {"artifacts": artifacts})
    return hashlib.sha256(canonicalize(value)).hexdigest()


def canonical_receipt_path(evidence_root: Path, supplied: Path) -> str:
    """Bind a non-symlink receipt destination beneath the evidence root."""
    root = evidence_root.resolve(strict=True)
    parent = supplied.parent.resolve(strict=True)
    receipt = parent / supplied.name
    if receipt == root or not receipt.is_relative_to(root):
        raise PortableReceiptNonceError("portable receipt path escapes evidence root")
    if receipt.is_symlink():
        raise PortableReceiptNonceError("portable receipt path is symlinked")
    return receipt.relative_to(root).as_posix()


def portable_receipt_nonce(claim: PortableReceiptClaim) -> str:
    """Derive the receipt nonce from the complete immutable claim identity."""
    value: JsonValue = {
        "domain": _NONCE_DOMAIN,
        "scope_sha256": claim.scope_sha256,
        "batch_id": claim.batch_id,
        "artifact_root_sha256": claim.artifact_root_sha256,
        "receipt_path": claim.receipt_path,
    }
    return hashlib.sha256(canonicalize(value)).hexdigest()


__all__ = [
    "PortableReceiptClaim",
    "PortableReceiptNonceError",
    "artifact_root_sha256",
    "canonical_receipt_path",
    "portable_receipt_nonce",
]
