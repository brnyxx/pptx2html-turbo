from __future__ import annotations

import hashlib
import shutil
from pathlib import Path

from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PublicKey

from evaluate.jcs import canonicalize
from evaluate.multiformat_capture_types import CaptureManifest
from evaluate.multiformat_review_packet_trust import review_pair_artifacts
from evaluate.multiformat_review_types import ReviewMaterializeError
from evaluate.multiformat_review_registry import (
    ReviewRegistryError,
    load_reviewer_registry,
)
from evaluate.multiformat_schema import JsonValue, sha256_file


def materialize_review_packet(
    output_dir: Path,
    oracle: CaptureManifest,
    candidate: CaptureManifest,
    expected_pairs: frozenset[str],
    *,
    bindings: dict[str, JsonValue],
) -> dict[str, JsonValue]:
    """Publishes a blank packet bound to the fixed reviewer registry.

    Reviewer identities, roles, and public keys are never caller-chosen: they
    come from the evaluator-bound registry, so a producer cannot introduce a
    reviewer whose private key it holds.
    """
    try:
        registry = load_reviewer_registry()
    except ReviewRegistryError as error:
        raise ReviewMaterializeError("reviewer registry is unusable") from error
    identities = tuple(
        (reviewer.reviewer_id, reviewer.reviewer_role, reviewer.public_key)
        for reviewer in registry.reviewers
    )
    for _reviewer_id, _role, key in identities:
        Ed25519PublicKey.from_public_bytes(key)
    if not expected_pairs:
        raise ReviewMaterializeError("review packet pair set is empty")
    output_dir.mkdir(parents=True, exist_ok=False)
    try:
        packet_path = output_dir / "review-packet.json"
        packet: dict[str, JsonValue] = {
            "schema_version": 2,
            "status": "READY",
            "checklist_version": "multiformat-review-v2",
            "bindings": bindings,
            "reviewers": [
                {
                    "reviewer_id": reviewer_id,
                    "reviewer_role": role,
                    "algorithm": "ed25519",
                    "public_key": key.hex(),
                    "public_key_sha256": hashlib.sha256(key).hexdigest(),
                }
                for reviewer_id, role, key in identities
            ],
            "pairs": [
                review_pair_artifacts(pair_id, oracle, candidate)
                for pair_id in sorted(expected_pairs)
            ],
        }
        packet_path.write_bytes(canonicalize(packet))
        packet_hash = sha256_file(packet_path)
        templates: list[JsonValue] = []
        for reviewer_id, role, key in identities:
            path = output_dir / f"decision-{reviewer_id}.json"
            path.write_bytes(
                canonicalize(
                    {
                        "schema_version": 2,
                        "packet_sha256": packet_hash,
                        "reviewer_id": reviewer_id,
                        "reviewer_role": role,
                        "public_key_sha256": hashlib.sha256(key).hexdigest(),
                        "checklist_version": "multiformat-review-v2",
                        "pairs": [
                            {
                                "pair_id": pair_id,
                                "decision": None,
                                "critical_defect": None,
                            }
                            for pair_id in sorted(expected_pairs)
                        ],
                    }
                )
            )
            templates.append(path.as_posix())
        return {
            "status": "INCOMPLETE",
            "review_packet": packet_path.as_posix(),
            "review_packet_sha256": packet_hash,
            "decision_templates": templates,
        }
    except BaseException:
        shutil.rmtree(output_dir)
        raise


__all__ = ["materialize_review_packet"]
