from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

from evaluate.multiformat_candidate_artifacts import (
    evidence_binding,
    write_canonical_json,
)
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
    string_value,
)
from evaluate.multiformat_strict_json import read_strict_object


class ReviewMaterializeError(RuntimeError):
    pass


@dataclass(frozen=True, slots=True)
class ReviewDecision:
    reviewer_id: str
    reviewer_role: str
    independent: bool
    decisions: dict[str, tuple[str, bool]]


def load_review_decision(
    path: Path,
    expected_pairs: frozenset[str],
) -> ReviewDecision:
    values = read_strict_object(path)
    require_keys(
        values,
        {
            "schema_version",
            "reviewer_id",
            "reviewer_role",
            "independent",
            "checklist_version",
            "pairs",
        },
        "review.decision",
    )
    if (
        integer_value(values, "schema_version") != 1
        or string_value(values, "checklist_version") != "multiformat-review-v1"
    ):
        raise ReviewMaterializeError("unsupported reviewer decision schema")
    decisions: dict[str, tuple[str, bool]] = {}
    for pair in object_list(values, "pairs", "review.decision.pairs"):
        require_keys(
            pair,
            {"pair_id", "decision", "critical_defect"},
            "review.decision.pair",
        )
        pair_id = string_value(pair, "pair_id")
        decision = string_value(pair, "decision")
        if pair_id in decisions or decision not in {"PASS", "FAIL"}:
            raise ReviewMaterializeError(f"invalid reviewer pair: {pair_id}")
        decisions[pair_id] = (
            decision,
            boolean_value(pair, "critical_defect"),
        )
    if set(decisions) != set(expected_pairs):
        raise ReviewMaterializeError("reviewer pair set is incomplete")
    return ReviewDecision(
        canonical_identity(string_value(values, "reviewer_id"), "reviewer_id"),
        canonical_identity(string_value(values, "reviewer_role"), "reviewer_role"),
        boolean_value(values, "independent"),
        decisions,
    )


def materialize_review_attestations(
    decision_paths: tuple[Path, ...],
    expected_pairs: frozenset[str],
    oracle: CaptureManifest,
    candidate: CaptureManifest,
    evaluator_hash: str,
    corpus_hash: str,
    project_revision: str,
    evidence_root: Path,
    output_dir: Path,
) -> tuple[list[JsonValue], dict[str, bool]]:
    if len(decision_paths) != 2:
        raise ReviewMaterializeError("exactly two reviewer decisions are required")
    reviews = tuple(
        load_review_decision(path, expected_pairs) for path in decision_paths
    )
    if (
        len({review.reviewer_id for review in reviews}) != 2
        or len({review.reviewer_role for review in reviews}) != 2
    ):
        raise ReviewMaterializeError("reviewer identities and roles must be distinct")
    critical = {
        pair_id: any(review.decisions[pair_id][1] for review in reviews)
        for pair_id in expected_pairs
    }
    bindings: list[JsonValue] = []
    for review in reviews:
        path = output_dir / f"{review.reviewer_id}.json"
        decisions = [review.decisions[pair_id][0] for pair_id in sorted(expected_pairs)]
        value: dict[str, JsonValue] = {
            "schema_version": 1,
            "status": (
                "PASS"
                if review.independent and all(item == "PASS" for item in decisions)
                else "FAIL"
            ),
            "reviewer_id": review.reviewer_id,
            "reviewer_role": review.reviewer_role,
            "independent": review.independent,
            "checklist_version": "multiformat-review-v1",
            "project_revision": project_revision,
            "evaluator_manifest_sha256": evaluator_hash,
            "corpus_manifest_sha256": corpus_hash,
            "pairs": [
                _pair_value(
                    pair_id,
                    review.decisions[pair_id][0],
                    oracle,
                    candidate,
                )
                for pair_id in sorted(expected_pairs)
            ],
        }
        path.parent.mkdir(parents=True, exist_ok=True)
        write_canonical_json(path, value)
        bindings.append({"attestation": evidence_binding(evidence_root, path)})
    return bindings, critical


def review_pair_artifacts(
    pair_id: str,
    oracle: CaptureManifest,
    candidate: CaptureManifest,
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


def _pair_value(
    pair_id: str,
    decision: str,
    oracle: CaptureManifest,
    candidate: CaptureManifest,
) -> dict[str, JsonValue]:
    return {
        **review_pair_artifacts(pair_id, oracle, candidate),
        "decision": decision,
    }
