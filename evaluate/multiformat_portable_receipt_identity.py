"""Verified portable receipt identities and replay finalization."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Final

from evaluate.multiformat_portable_receipt_nonce import artifact_root_sha256
from evaluate.multiformat_portable_receipt_trust import PortableReceiptTrustContext
from evaluate.multiformat_portable_receipt_validation import StableFileIdentity
from evaluate.multiformat_schema import JsonValue


class ReceiptIdentitySeal:
    """Unforgeable marker; only this module's constant marks verification."""

    __slots__ = ()


_IDENTITY_SEAL: Final = ReceiptIdentitySeal()


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
    _seal: ReceiptIdentitySeal

    def is_verified(self) -> bool:
        return self._seal is _IDENTITY_SEAL


VerifiedPortableReceipt = PortableReceiptIdentity


@dataclass(frozen=True, slots=True)
class PortableReceiptVerification:
    trust: PortableReceiptTrustContext
    bound_receipt_path: Path | None = None


def finalize_receipt_identity(
    *,
    trust: PortableReceiptTrustContext,
    payload_sha256: str,
    nonce: str,
    batch_id: str,
    artifacts: list[dict[str, JsonValue]],
    stable_artifacts: tuple[StableFileIdentity, ...],
) -> PortableReceiptIdentity:
    """Seal an identity after its claim-derived nonce has been verified."""
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
    return identity


__all__ = [
    "PortableReceiptIdentity",
    "PortableReceiptVerification",
    "ReceiptIdentitySeal",
    "VerifiedPortableReceipt",
    "finalize_receipt_identity",
]
