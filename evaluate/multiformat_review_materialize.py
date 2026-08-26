from __future__ import annotations

import hashlib
from dataclasses import dataclass
from pathlib import Path

from cryptography.exceptions import InvalidSignature
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PublicKey

from evaluate.jcs import canonicalize
from evaluate.multiformat_candidate_artifacts import evidence_binding
from evaluate.multiformat_capture_types import CaptureManifest
from evaluate.multiformat_corpus_items import (
    canonical_identity,
    object_list,
    require_keys,
)
from evaluate.multiformat_schema import (
    JsonValue,
    boolean_value,
    integer_value,
    sha256_file,
    sha256_value,
    string_value,
)
from evaluate.multiformat_strict_json import read_strict_object


class ReviewMaterializeError(RuntimeError):
    pass


def _hex_bytes(values: dict[str, JsonValue], name: str, length: int) -> bytes:
    try:
        result = bytes.fromhex(string_value(values, name))
    except ValueError as error:
        raise ReviewMaterializeError(f"{name} is not hexadecimal") from error
    if len(result) != length:
        raise ReviewMaterializeError(f"{name} has invalid length")
    return result


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
    if {string_value(pair, "pair_id") for pair in pairs} != set(expected_pairs) or len(
        pairs
    ) != len(expected_pairs):
        raise ReviewMaterializeError("review packet pair set differs")
    for pair in pairs:
        pair_id = string_value(pair, "pair_id")
        if pair != review_pair_artifacts(pair_id, oracle, candidate):
            raise ReviewMaterializeError(
                f"review packet artifact scope differs: {pair_id}"
            )
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
        key = _hex_bytes(value, "public_key", 32)
        digest = hashlib.sha256(key).hexdigest()
        if (
            string_value(value, "algorithm") != "ed25519"
            or sha256_value(value, "public_key_sha256") != digest
            or reviewer_id in trusts
            or role in roles
            or key in keys
        ):
            raise ReviewMaterializeError(
                "review packet reviewer trust is invalid or duplicated"
            )
        trusts[reviewer_id] = ReviewerTrust(reviewer_id, role, key, digest)
        roles.add(role)
        keys.add(key)
    if len(trusts) != 2:
        raise ReviewMaterializeError(
            "review packet requires two distinct reviewer keys"
        )
    return trusts, sha256_file(path)


def load_review_decision(
    path: Path, expected_pairs: frozenset[str], packet_sha256: str, trust: ReviewerTrust
) -> ReviewDecision:
    values = read_strict_object(path)
    signature_value = values.pop("signature", None)
    require_keys(
        values,
        {
            "schema_version",
            "packet_sha256",
            "reviewer_id",
            "reviewer_role",
            "public_key_sha256",
            "checklist_version",
            "pairs",
        },
        "review.decision",
    )
    try:
        if (
            integer_value(values, "schema_version") != 2
            or string_value(values, "checklist_version") != "multiformat-review-v2"
            or sha256_value(values, "packet_sha256") != packet_sha256
            or string_value(values, "reviewer_id") != trust.reviewer_id
            or string_value(values, "reviewer_role") != trust.reviewer_role
            or sha256_value(values, "public_key_sha256") != trust.public_key_sha256
        ):
            raise ReviewMaterializeError("signed reviewer identity or packet differs")
        if not isinstance(signature_value, str):
            raise ReviewMaterializeError("review signature is missing")
        Ed25519PublicKey.from_public_bytes(trust.public_key).verify(
            bytes.fromhex(signature_value), canonicalize(values)
        )
    except (InvalidSignature, ValueError) as error:
        raise ReviewMaterializeError("review signature verification failed") from error
    decisions: dict[str, tuple[str, bool]] = {}
    for pair in object_list(values, "pairs", "review.decision.pairs"):
        require_keys(
            pair, {"pair_id", "decision", "critical_defect"}, "review.decision.pair"
        )
        pair_id = string_value(pair, "pair_id")
        decision = string_value(pair, "decision")
        if pair_id in decisions or decision not in {"PASS", "FAIL"}:
            raise ReviewMaterializeError(f"invalid reviewer pair: {pair_id}")
        decisions[pair_id] = (decision, boolean_value(pair, "critical_defect"))
    if set(decisions) != set(expected_pairs):
        raise ReviewMaterializeError("reviewer pair set is incomplete")
    return ReviewDecision(
        trust.reviewer_id, trust.reviewer_role, trust.public_key_sha256, decisions
    )


def materialize_review_attestations(
    decision_paths: tuple[Path, ...],
    packet_path: Path,
    expected_pairs: frozenset[str],
    oracle: CaptureManifest,
    candidate: CaptureManifest,
    bindings: dict[str, JsonValue],
    evidence_root: Path,
) -> tuple[dict[str, JsonValue], dict[str, bool]]:
    if len(decision_paths) != 2:
        raise ReviewMaterializeError(
            "exactly two signed reviewer decisions are required"
        )
    trusts, packet_hash = load_review_packet(
        packet_path, expected_pairs, oracle, candidate, bindings
    )
    reviews: list[ReviewDecision] = []
    paths: set[Path] = set()
    for path in decision_paths:
        raw = read_strict_object(path)
        reviewer_id = string_value(raw, "reviewer_id")
        trust = trusts.get(reviewer_id)
        if trust is None or path.resolve(strict=True) in paths:
            raise ReviewMaterializeError(
                "review decision signer is not packet-bound or is duplicated"
            )
        paths.add(path.resolve(strict=True))
        reviews.append(load_review_decision(path, expected_pairs, packet_hash, trust))
    if {review.reviewer_id for review in reviews} != set(trusts):
        raise ReviewMaterializeError("review decisions do not cover packet reviewers")
    critical = {
        pair_id: any(review.decisions[pair_id][1] for review in reviews)
        for pair_id in expected_pairs
    }
    return (
        {
            "packet": evidence_binding(evidence_root, packet_path),
            "decisions": [
                evidence_binding(evidence_root, path) for path in decision_paths
            ],
        },
        critical,
    )


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
