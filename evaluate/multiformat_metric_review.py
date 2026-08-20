from __future__ import annotations

from pathlib import Path

from evaluate.multiformat_corpus_items import (
    canonical_identity,
    object_list,
    require_keys,
)
from evaluate.multiformat_metric_compute import resolve_artifact_binding
from evaluate.multiformat_capture_types import CaptureManifest
from evaluate.multiformat_metric_types import CorpusMetricSpec, MetricError
from evaluate.multiformat_schema import (
    JsonValue,
    boolean_value,
    integer_value,
    object_value,
    sha256_value,
    string_value,
)
from evaluate.multiformat_strict_json import read_strict_object


def compute_review(
    values: dict[str, JsonValue],
    spec: CorpusMetricSpec,
    evidence_root: Path,
    oracle: CaptureManifest,
    candidate: CaptureManifest,
    evaluator_hash: str,
    corpus_hash: str,
    project_revision: str,
) -> tuple[int, bool, set[Path]]:
    require_keys(values, {"reviewers"}, "review")
    records = object_list(values, "reviewers", "review.reviewers")
    expected_pairs = spec.pair_ids()
    identities: set[str] = set()
    roles: set[str] = set()
    attestations: set[Path] = set()
    all_passed = len(records) == 2
    for record in records:
        require_keys(record, {"attestation"}, "review.reviewer")
        path = resolve_artifact_binding(
            object_value(record, "attestation"),
            evidence_root,
            "review.attestation",
        )
        if path in attestations:
            raise MetricError("review.reviewer_set", path.as_posix())
        attestations.add(path)
        reviewer = read_strict_object(path)
        require_keys(
            reviewer,
            {
                "schema_version",
                "status",
                "reviewer_id",
                "reviewer_role",
                "independent",
                "checklist_version",
                "project_revision",
                "evaluator_manifest_sha256",
                "corpus_manifest_sha256",
                "pairs",
            },
            "review.attestation",
        )
        reviewer_id = canonical_identity(
            string_value(reviewer, "reviewer_id"),
            "review.reviewer_id",
        )
        role = string_value(reviewer, "reviewer_role")
        if reviewer_id in identities or role in roles:
            raise MetricError("review.reviewer_set", reviewer_id)
        identities.add(reviewer_id)
        roles.add(role)
        revision = string_value(reviewer, "project_revision")
        decisions: dict[str, str] = {}
        for pair in object_list(reviewer, "pairs", "review.pairs"):
            require_keys(
                pair,
                {
                    "pair_id",
                    "reference_png_sha256",
                    "candidate_png_sha256",
                    "reference_inventory_sha256",
                    "candidate_inventory_sha256",
                    "decision",
                },
                "review.pair",
            )
            pair_id = string_value(pair, "pair_id")
            if pair_id in decisions or not _pair_hashes_match(
                pair,
                pair_id,
                oracle,
                candidate,
            ):
                raise MetricError("review.pair_set", pair_id)
            decisions[pair_id] = string_value(pair, "decision")
        all_passed &= all(
            [
                integer_value(reviewer, "schema_version") == 1,
                string_value(reviewer, "status") == "PASS",
                boolean_value(reviewer, "independent"),
                string_value(reviewer, "checklist_version") == "multiformat-review-v1",
                revision == project_revision,
                sha256_value(reviewer, "evaluator_manifest_sha256") == evaluator_hash,
                sha256_value(reviewer, "corpus_manifest_sha256") == corpus_hash,
                set(decisions) == set(expected_pairs),
                all(decision == "PASS" for decision in decisions.values()),
            ]
        )
    return len(records), all_passed, attestations


def _pair_hashes_match(
    pair: dict[str, JsonValue],
    pair_id: str,
    oracle: CaptureManifest,
    candidate: CaptureManifest,
) -> bool:
    if pair_id not in oracle.units or pair_id not in candidate.units:
        return False
    return all(
        [
            sha256_value(pair, "reference_png_sha256")
            == oracle.units[pair_id].png.sha256,
            sha256_value(pair, "candidate_png_sha256")
            == candidate.units[pair_id].png.sha256,
            sha256_value(pair, "reference_inventory_sha256")
            == oracle.units[pair_id].inventory.sha256,
            sha256_value(pair, "candidate_inventory_sha256")
            == candidate.units[pair_id].inventory.sha256,
        ]
    )
