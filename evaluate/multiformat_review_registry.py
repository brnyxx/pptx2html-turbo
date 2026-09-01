"""Evaluator-bound reviewer trust anchor.

The tracked registry, not the review packet, decides which two independent
reviewers exist. Producers only ever hold the public keys, so a producer
cannot author a second reviewer identity and then have the evaluator trust it.
Packet materialization, decision validation, and metrics all re-load this file
independently instead of believing packet-provided keys.

There is deliberately no environment variable, configuration key, or mutable
module state that can redirect the trust anchor: production always resolves
``REGISTRY_PATH``. Unit tests that need throwaway keypairs patch the
consumer-side loader boundary instead.
"""

from __future__ import annotations

import hashlib
from dataclasses import dataclass
from pathlib import Path

from evaluate.multiformat_corpus_items import (
    canonical_identity,
    object_list,
    require_keys,
)
from evaluate.multiformat_corpus_types import CorpusError
from evaluate.multiformat_schema import (
    JsonValue,
    integer_value,
    sha256_value,
    string_value,
)
from evaluate.multiformat_strict_json import StrictJsonError, read_strict_object

REGISTRY_PATH = (
    Path(__file__).resolve().parent / "multiformat" / "reviewer-registry.v1.json"
)
REGISTRY_SCHEMA_VERSION = 1
REVIEWER_ALGORITHM = "ed25519"
REVIEWER_COUNT = 2
PUBLIC_KEY_BYTES = 32


class ReviewRegistryError(RuntimeError):
    pass


@dataclass(frozen=True, slots=True)
class RegisteredReviewer:
    reviewer_id: str
    reviewer_role: str
    public_key: bytes
    public_key_sha256: str


@dataclass(frozen=True, slots=True)
class ReviewerRegistry:
    reviewers: tuple[RegisteredReviewer, ...]

    def by_id(self) -> dict[str, RegisteredReviewer]:
        return {reviewer.reviewer_id: reviewer for reviewer in self.reviewers}

    def identities(self) -> tuple[tuple[str, str], ...]:
        return tuple(
            (reviewer.reviewer_id, reviewer.reviewer_role)
            for reviewer in self.reviewers
        )


def load_reviewer_registry(path: Path = REGISTRY_PATH) -> ReviewerRegistry:
    """Loads and fully validates a reviewer registry.

    Production callers use the default fixed path. The parameter exists so a
    registry can be validated in place before it is committed, and so unit
    tests can build a temporary registry explicitly; it is never wired to any
    production CLI flag or environment lookup.
    """
    try:
        values = read_strict_object(path)
        require_keys(values, {"schema_version", "algorithm", "reviewers"}, "registry")
        if (
            integer_value(values, "schema_version") != REGISTRY_SCHEMA_VERSION
            or string_value(values, "algorithm") != REVIEWER_ALGORITHM
        ):
            raise ReviewRegistryError("reviewer registry schema is unsupported")
        entries = object_list(values, "reviewers", "registry.reviewers")
        reviewers = tuple(_registered_reviewer(entry, path.parent) for entry in entries)
    except (CorpusError, StrictJsonError, OSError, TypeError, ValueError) as error:
        raise ReviewRegistryError("reviewer registry is invalid") from error
    if (
        len(reviewers) != REVIEWER_COUNT
        or len({reviewer.reviewer_id for reviewer in reviewers}) != REVIEWER_COUNT
        or len({reviewer.reviewer_role for reviewer in reviewers}) != REVIEWER_COUNT
        or len({reviewer.public_key for reviewer in reviewers}) != REVIEWER_COUNT
    ):
        raise ReviewRegistryError(
            "reviewer registry requires two distinct reviewers, roles, and keys"
        )
    return ReviewerRegistry(reviewers)


def _registered_reviewer(entry: dict[str, JsonValue], root: Path) -> RegisteredReviewer:
    require_keys(
        entry,
        {
            "reviewer_id",
            "reviewer_role",
            "algorithm",
            "public_key_path",
            "public_key_sha256",
        },
        "registry.reviewer",
    )
    if string_value(entry, "algorithm") != REVIEWER_ALGORITHM:
        raise ReviewRegistryError("reviewer registry algorithm is unsupported")
    relative = string_value(entry, "public_key_path")
    key = _public_key_bytes(root, relative)
    digest = hashlib.sha256(key).hexdigest()
    if sha256_value(entry, "public_key_sha256") != digest:
        raise ReviewRegistryError(f"reviewer public key digest differs: {relative}")
    return RegisteredReviewer(
        canonical_identity(string_value(entry, "reviewer_id"), "reviewer_id"),
        canonical_identity(string_value(entry, "reviewer_role"), "reviewer_role"),
        key,
        digest,
    )


def _public_key_bytes(root: Path, relative: str) -> bytes:
    candidate = Path(relative)
    if candidate.is_absolute() or ".." in candidate.parts:
        raise ReviewRegistryError("reviewer public key path escapes the registry")
    target = root / candidate
    if target.is_symlink():
        raise ReviewRegistryError("reviewer public key path escapes the registry")
    resolved = target.resolve(strict=True)
    if not resolved.is_relative_to(root.resolve(strict=True)):
        raise ReviewRegistryError("reviewer public key path escapes the registry")
    key = resolved.read_bytes()
    if len(key) != PUBLIC_KEY_BYTES:
        raise ReviewRegistryError("reviewer public key must be raw 32-byte Ed25519")
    return key


__all__ = [
    "PUBLIC_KEY_BYTES",
    "REGISTRY_PATH",
    "RegisteredReviewer",
    "ReviewRegistryError",
    "ReviewerRegistry",
    "load_reviewer_registry",
]
