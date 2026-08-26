"""Trusted signed portable-reference receipt boundary."""

from __future__ import annotations

import hashlib
from dataclasses import dataclass
from pathlib import Path
from typing import Final, TypeAlias, cast

from cryptography.exceptions import InvalidSignature
from cryptography.hazmat.primitives.asymmetric.ed25519 import (
    Ed25519PrivateKey,
    Ed25519PublicKey,
)

from evaluate.jcs import JcsError, canonicalize
from evaluate.multiformat_evidence import EvidencePathError
from evaluate.multiformat_portable_receipt_identity import (
    PortableReceiptIdentity,
    PortableReceiptVerification,
    finalize_receipt_identity,
)
from evaluate.multiformat_portable_receipt_nonce import (
    PortableReceiptClaim,
    PortableReceiptNonceError,
    artifact_root_sha256,
    canonical_receipt_path,
    portable_receipt_nonce,
)
from evaluate.multiformat_portable_receipt_trust import (
    PortableReceiptTrustContext,
    PortableReceiptTrustError,
    runtime_record,
    verify_trusted_files,
)
from evaluate.multiformat_portable_receipt_validation import (
    ReceiptValidationError,
    hex_bytes,
    object_array,
    reject_identity_aliases,
    require_exact_keys,
    validate_artifact_records,
    verify_artifacts,
)
from evaluate.multiformat_schema import (
    JsonValue,
    integer_value,
    object_value,
    sha256_value,
    string_value,
)
from evaluate.multiformat_strict_json import StrictJsonError, read_strict_object

JsonObject: TypeAlias = dict[str, JsonValue]
_ALGORITHM: Final = "ed25519"
_ALGORITHM_VERSION: Final = 1
_SCHEMA_VERSION: Final = 2


VerifiedPortableReceipt = PortableReceiptIdentity


class PortableReceiptError(ValueError):
    """Typed portable receipt boundary failure."""

    __slots__ = ("reason",)

    def __init__(self, reason: str) -> None:
        self.reason = reason
        super().__init__(reason)


@dataclass(frozen=True, slots=True)
class PortableReceiptInput:
    trust: PortableReceiptTrustContext
    batch_id: str
    artifacts: list[JsonObject]
    bound_receipt_path: Path | None = None


def sign_portable_receipt(
    output: Path,
    receipt_input: PortableReceiptInput,
    private_key: Ed25519PrivateKey,
) -> Path:
    """Sign digest bytes for a runtime derived only from validated trust."""
    try:
        trust = receipt_input.trust
        _require_trust(trust)
        if not receipt_input.batch_id:
            raise PortableReceiptError("portable receipt batch identity is empty")
        validate_artifact_records(receipt_input.artifacts)
        if private_key.public_key().public_bytes_raw() != trust.public_key:
            raise PortableReceiptError("portable receipt signing key is not trusted")
        nonce = portable_receipt_nonce(
            PortableReceiptClaim(
                scope_sha256=trust.scope_sha256,
                batch_id=receipt_input.batch_id,
                artifact_root_sha256=artifact_root_sha256(receipt_input.artifacts),
                receipt_path=canonical_receipt_path(
                    trust.evidence_root,
                    receipt_input.bound_receipt_path or output,
                ),
            )
        )
        runtime = runtime_record(trust, nonce, receipt_input.batch_id)
        payload = _payload(runtime, receipt_input.artifacts)
        digest = hashlib.sha256(canonicalize(payload)).digest()
        receipt: JsonObject = {
            "schema_version": _SCHEMA_VERSION,
            "algorithm": _ALGORITHM,
            "algorithm_version": _ALGORITHM_VERSION,
            "signer_identity": trust.signer_identity,
            "public_key": trust.public_key.hex(),
            "public_key_sha256": trust.public_key_sha256,
            "payload_sha256": digest.hex(),
            "signature": private_key.sign(digest).hex(),
            **payload,
        }
        output.write_bytes(canonicalize(receipt))
        return output
    except PortableReceiptError:
        raise
    except (
        JcsError,
        OSError,
        PortableReceiptTrustError,
        TypeError,
        ValueError,
    ) as error:
        raise PortableReceiptError(
            "portable receipt signing input is invalid"
        ) from error


def verify_portable_receipt(
    receipt_path: Path,
    verification: PortableReceiptVerification,
) -> PortableReceiptIdentity:
    """Verify one receipt against a sealed lock-derived trust context."""
    try:
        trust = verification.trust
        _require_trust(trust)
        receipt_bytes = receipt_path.read_bytes()
        receipt = read_strict_object(receipt_path)
        if receipt_bytes != canonicalize(receipt):
            raise PortableReceiptError("portable receipt is not canonical JCS")
        _validate_envelope(receipt, trust)
        runtime = object_value(receipt, "runtime")
        artifacts = object_array(receipt, "artifacts")
        nonce = sha256_value(runtime, "nonce")
        batch_id = string_value(runtime, "batch_id")
        validate_artifact_records(artifacts)
        expected_nonce = portable_receipt_nonce(
            PortableReceiptClaim(
                scope_sha256=trust.scope_sha256,
                batch_id=batch_id,
                artifact_root_sha256=artifact_root_sha256(artifacts),
                receipt_path=canonical_receipt_path(
                    trust.evidence_root,
                    verification.bound_receipt_path or receipt_path,
                ),
            )
        )
        if nonce != expected_nonce:
            raise PortableReceiptError("portable receipt deterministic nonce differs")
        if runtime != runtime_record(trust, nonce, batch_id):
            raise PortableReceiptError("portable receipt signed scope differs")
        digest = hashlib.sha256(canonicalize(_payload(runtime, artifacts))).digest()
        if sha256_value(receipt, "payload_sha256") != digest.hex():
            raise PortableReceiptError("portable receipt payload digest differs")
        verify_trusted_files(trust)
        signature = hex_bytes(receipt, "signature", 64)
        Ed25519PublicKey.from_public_bytes(trust.public_key).verify(signature, digest)
        stable_artifacts = verify_artifacts(artifacts, trust.evidence_root)
        final_lock, final_sources = verify_trusted_files(trust)
        reject_identity_aliases((final_lock, final_sources, stable_artifacts))
        return finalize_receipt_identity(
            trust=trust,
            payload_sha256=digest.hex(),
            nonce=nonce,
            batch_id=batch_id,
            artifacts=artifacts,
            stable_artifacts=stable_artifacts,
        )
    except PortableReceiptError:
        raise
    except (
        PortableReceiptNonceError,
        ReceiptValidationError,
        PortableReceiptTrustError,
    ) as error:
        raise PortableReceiptError(str(error)) from error
    except (
        EvidencePathError,
        InvalidSignature,
        JcsError,
        OSError,
        StrictJsonError,
        TypeError,
        UnicodeError,
        ValueError,
    ) as error:
        raise PortableReceiptError("portable receipt verification failed") from error


def _validate_envelope(
    receipt: JsonObject,
    trust: PortableReceiptTrustContext,
) -> None:
    require_exact_keys(
        receipt,
        {
            "schema_version",
            "algorithm",
            "algorithm_version",
            "signer_identity",
            "public_key",
            "public_key_sha256",
            "payload_sha256",
            "signature",
            "runtime",
            "artifacts",
        },
        "receipt",
    )
    if (
        integer_value(receipt, "schema_version") != _SCHEMA_VERSION
        or string_value(receipt, "algorithm") != _ALGORITHM
        or integer_value(receipt, "algorithm_version") != _ALGORITHM_VERSION
        or string_value(receipt, "signer_identity") != trust.signer_identity
        or hex_bytes(receipt, "public_key", 32) != trust.public_key
        or sha256_value(receipt, "public_key_sha256") != trust.public_key_sha256
    ):
        raise PortableReceiptError("portable receipt trusted envelope differs")


def _require_trust(trust: PortableReceiptTrustContext) -> None:
    if not isinstance(trust, PortableReceiptTrustContext) or not trust.is_valid():
        raise PortableReceiptError("portable receipt trust identity is invalid")


def _payload(runtime: JsonObject, artifacts: list[JsonObject]) -> JsonObject:
    return {"runtime": runtime, "artifacts": cast(JsonValue, artifacts)}
