"""Shared review error type, trust records, and hex decoding.

Kept separate so packet-trust resolution and decision loading can both depend
on these definitions without importing each other.
"""

from __future__ import annotations

from dataclasses import dataclass

from evaluate.multiformat_schema import JsonValue, string_value


class ReviewMaterializeError(RuntimeError):
    pass


@dataclass(frozen=True, slots=True)
class ReviewerTrust:
    reviewer_id: str
    reviewer_role: str
    public_key: bytes
    public_key_sha256: str


@dataclass(frozen=True, slots=True)
class ReviewDecision:
    reviewer_id: str
    reviewer_role: str
    public_key_sha256: str
    decisions: dict[str, tuple[str, bool]]


def hex_bytes(values: dict[str, JsonValue], name: str, length: int) -> bytes:
    try:
        result = bytes.fromhex(string_value(values, name))
    except ValueError as error:
        raise ReviewMaterializeError(f"{name} is not hexadecimal") from error
    if len(result) != length:
        raise ReviewMaterializeError(f"{name} has invalid length")
    return result


__all__ = [
    "ReviewDecision",
    "ReviewMaterializeError",
    "ReviewerTrust",
    "hex_bytes",
]
