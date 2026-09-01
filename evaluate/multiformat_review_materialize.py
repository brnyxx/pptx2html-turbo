"""Signed reviewer decision loading and review attestation materialization.

Packet-scope verification and registry-bound trust resolution live in
``multiformat_review_packet_trust``; both are re-exported here so existing
callers keep a single import surface.
"""

from __future__ import annotations

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
from evaluate.multiformat_review_packet_trust import (
    load_review_packet,
    packet_reviewer_trusts,
    review_pair_artifacts,
    reviewer_trusts,
)
from evaluate.multiformat_review_types import (
    ReviewDecision,
    ReviewerTrust,
    ReviewMaterializeError,
)
from evaluate.multiformat_schema import (
    JsonValue,
    boolean_value,
    integer_value,
    sha256_value,
    string_value,
)
from evaluate.multiformat_strict_json import read_strict_object


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
    reviewer_id = canonical_identity(string_value(values, "reviewer_id"), "reviewer_id")
    reviewer_role = canonical_identity(
        string_value(values, "reviewer_role"), "reviewer_role"
    )
    try:
        if (
            integer_value(values, "schema_version") != 2
            or string_value(values, "checklist_version") != "multiformat-review-v2"
            or sha256_value(values, "packet_sha256") != packet_sha256
            or reviewer_id != trust.reviewer_id
            or reviewer_role != trust.reviewer_role
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
        reviewer_id = canonical_identity(
            string_value(raw, "reviewer_id"), "reviewer_id"
        )
        trust = trusts.get(reviewer_id)
        if trust is None or path.resolve(strict=True) in paths:
            raise ReviewMaterializeError(
                "review decision signer is not registry-bound or is duplicated"
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


__all__ = [
    "ReviewDecision",
    "ReviewMaterializeError",
    "ReviewerTrust",
    "load_review_decision",
    "load_review_packet",
    "materialize_review_attestations",
    "packet_reviewer_trusts",
    "review_pair_artifacts",
    "reviewer_trusts",
]
