"""Verified portable receipt identities and replay finalization."""

from __future__ import annotations

import hashlib
from dataclasses import dataclass
from pathlib import Path
from typing import Final

from evaluate.multiformat_portable_receipt_replay import (
    artifact_root_sha256,
    claim_portable_receipt,
)
from evaluate.multiformat_portable_receipt_trust import PortableReceiptTrustContext
from evaluate.multiformat_portable_receipt_validation import StableFileIdentity
from evaluate.multiformat_schema import JsonValue


class PortableReceiptIdentityError(ValueError):
    """A receipt identity or caller-provided replay identity is invalid."""


class _ReceiptIdentitySeal:
    __slots__ = ()


_IDENTITY_SEAL: Final = _ReceiptIdentitySeal()


@dataclass(frozen=True, slots=True)
class PortableReceiptIdentity:
    payload_sha256: str
    public_key_sha256: str
    nonce: str
    batch_id: str
    signer_identity: str
    scope_sha256: str
    artifact_root_sha256: str
    artifacts: tuple[StableFileIdentity, ...]
    _seal: _ReceiptIdentitySeal

    def is_verified(self) -> bool:
        return self._seal is _IDENTITY_SEAL


VerifiedPortableReceipt = PortableReceiptIdentity


@dataclass(frozen=True, slots=True)
class PortableReceiptVerification:
    trust: PortableReceiptTrustContext
    prior_receipts: tuple[PortableReceiptIdentity, ...] = ()
    claim_replay: bool = True
    bound_receipt_path: Path | None = None


def reject_prior_replay(
    nonce: str,
    scope_sha256: str,
    prior_receipts: tuple[PortableReceiptIdentity, ...],
) -> None:
    for identity in prior_receipts:
        if (
            not isinstance(identity, PortableReceiptIdentity)
            or not identity.is_verified()
        ):
            raise PortableReceiptIdentityError(
                "portable replay input is not a verified identity"
            )
        if identity.scope_sha256 == scope_sha256 and identity.nonce == nonce:
            raise PortableReceiptIdentityError("portable receipt nonce was replayed")


def finalize_receipt_identity(
    *,
    trust: PortableReceiptTrustContext,
    receipt_path: Path,
    bound_receipt_path: Path | None,
    receipt_bytes: bytes,
    payload_sha256: str,
    nonce: str,
    batch_id: str,
    artifacts: list[dict[str, JsonValue]],
    stable_artifacts: tuple[StableFileIdentity, ...],
    claim_replay: bool,
) -> PortableReceiptIdentity:
    """Seal a verified identity and atomically publish its replay claim."""
    artifact_root = artifact_root_sha256(artifacts)
    identity = PortableReceiptIdentity(
        payload_sha256=payload_sha256,
        public_key_sha256=trust.public_key_sha256,
        nonce=nonce,
        batch_id=batch_id,
        signer_identity=trust.signer_identity,
        scope_sha256=trust.scope_sha256,
        artifact_root_sha256=artifact_root,
        artifacts=stable_artifacts,
        _seal=_IDENTITY_SEAL,
    )
    if claim_replay:
        claim_portable_receipt(
            evidence_root=trust.evidence_root,
            receipt_path=bound_receipt_path or receipt_path,
            receipt_sha256=hashlib.sha256(receipt_bytes).hexdigest(),
            scope_sha256=trust.scope_sha256,
            nonce=nonce,
            batch_id=batch_id,
            artifact_root=artifact_root,
        )
    return identity


__all__ = [
    "PortableReceiptIdentity",
    "PortableReceiptIdentityError",
    "PortableReceiptVerification",
    "VerifiedPortableReceipt",
    "finalize_receipt_identity",
    "reject_prior_replay",
]
