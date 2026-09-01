"""Packet scope checks and registry-bound reviewer trust resolution.

Split out of ``multiformat_review_materialize`` so that packet-scope
verification and trust anchoring stay one cohesive unit, separate from signed
decision loading and attestation materialization.
"""

from __future__ import annotations

import hashlib
from pathlib import Path

from evaluate.multiformat_capture_types import CaptureManifest
from evaluate.multiformat_corpus_items import (
    canonical_identity,
    object_list,
    require_keys,
)
from evaluate.multiformat_review_types import (
    ReviewerTrust,
    ReviewMaterializeError,
    hex_bytes,
)
from evaluate.multiformat_review_registry import (
    ReviewerRegistry,
    ReviewRegistryError,
    load_reviewer_registry,
)
from evaluate.multiformat_schema import (
    JsonValue,
    integer_value,
    sha256_file,
    sha256_value,
    string_value,
)
from evaluate.multiformat_strict_json import read_strict_object

PUBLIC_KEY_BYTES = 32


def load_review_packet(
    path: Path,
    expected_pairs: frozenset[str],
    oracle: CaptureManifest,
    candidate: CaptureManifest,
    bindings: dict[str, JsonValue],
) -> tuple[dict[str, ReviewerTrust], str]:
    values = read_strict_object(path)
    require_keys(
        values,
        {
            "schema_version",
            "status",
            "checklist_version",
            "bindings",
            "reviewers",
            "pairs",
        },
        "review.packet",
    )
    if (
        integer_value(values, "schema_version") != 2
        or string_value(values, "status") != "READY"
        or string_value(values, "checklist_version") != "multiformat-review-v2"
        or values["bindings"] != bindings
    ):
        raise ReviewMaterializeError("review packet scope differs")
    pairs = object_list(values, "pairs", "review.packet.pairs")
    identifiers = [string_value(pair, "pair_id") for pair in pairs]
    if set(identifiers) != set(expected_pairs) or len(pairs) != len(expected_pairs):
        raise ReviewMaterializeError("review packet pair set differs")
    for pair_id, pair in zip(identifiers, pairs, strict=True):
        if pair != review_pair_artifacts(pair_id, oracle, candidate):
            raise ReviewMaterializeError(
                f"review packet artifact scope differs: {pair_id}"
            )
    return reviewer_trusts(values), sha256_file(path)


def reviewer_trusts(values: dict[str, JsonValue]) -> dict[str, ReviewerTrust]:
    """Resolves packet reviewers against the independently loaded registry.

    The packet is only allowed to restate what the registry already fixes. Any
    substituted, swapped, or self-authored reviewer key fails closed, because
    trust comes from the registry file and not from the packet bytes.
    """
    try:
        registry = load_reviewer_registry()
    except ReviewRegistryError as error:
        raise ReviewMaterializeError("reviewer registry is unusable") from error
    return packet_reviewer_trusts(values, registry)


def packet_reviewer_trusts(
    values: dict[str, JsonValue], registry: ReviewerRegistry
) -> dict[str, ReviewerTrust]:
    expected = registry.by_id()
    trusts: dict[str, ReviewerTrust] = {}
    roles: set[str] = set()
    keys: set[bytes] = set()
    for value in object_list(values, "reviewers", "review.packet.reviewers"):
        require_keys(
            value,
            {
                "reviewer_id",
                "reviewer_role",
                "algorithm",
                "public_key",
                "public_key_sha256",
            },
            "review.packet.reviewer",
        )
        reviewer_id = canonical_identity(
            string_value(value, "reviewer_id"), "reviewer_id"
        )
        role = canonical_identity(string_value(value, "reviewer_role"), "reviewer_role")
        key = hex_bytes(value, "public_key", PUBLIC_KEY_BYTES)
        digest = hashlib.sha256(key).hexdigest()
        registered = expected.get(reviewer_id)
        if (
            string_value(value, "algorithm") != "ed25519"
            or sha256_value(value, "public_key_sha256") != digest
            or reviewer_id in trusts
            or role in roles
            or key in keys
            or registered is None
            or registered.reviewer_role != role
            or registered.public_key != key
        ):
            raise ReviewMaterializeError(
                "review packet reviewer trust is not registry-bound"
            )
        trusts[reviewer_id] = ReviewerTrust(reviewer_id, role, key, digest)
        roles.add(role)
        keys.add(key)
    if set(trusts) != set(expected):
        raise ReviewMaterializeError(
            "review packet must carry exactly the registered reviewers"
        )
    return trusts


def review_pair_artifacts(
    pair_id: str, oracle: CaptureManifest, candidate: CaptureManifest
) -> dict[str, JsonValue]:
    if pair_id not in oracle.units or pair_id not in candidate.units:
        raise ReviewMaterializeError(f"capture pair is missing: {pair_id}")
    reference = oracle.units[pair_id]
    captured = candidate.units[pair_id]
    return {
        "pair_id": pair_id,
        "reference_png_sha256": reference.png.sha256,
        "candidate_png_sha256": captured.png.sha256,
        "reference_inventory_sha256": reference.inventory.sha256,
        "candidate_inventory_sha256": captured.inventory.sha256,
    }


__all__ = [
    "load_review_packet",
    "packet_reviewer_trusts",
    "review_pair_artifacts",
    "reviewer_trusts",
]
