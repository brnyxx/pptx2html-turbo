from __future__ import annotations

from pathlib import Path

from evaluate.multiformat_capture_types import CaptureManifest
from evaluate.multiformat_corpus_items import object_list, require_keys
from evaluate.multiformat_metric_compute import resolve_artifact_binding
from evaluate.multiformat_metric_types import CorpusMetricSpec, MetricError
from evaluate.multiformat_review_materialize import (
    ReviewMaterializeError,
    load_review_decision,
    load_review_packet,
)
from evaluate.multiformat_schema import JsonValue, object_value, string_value
from evaluate.multiformat_strict_json import read_strict_object


def compute_review(
    values: dict[str, JsonValue],
    spec: CorpusMetricSpec,
    evidence_root: Path,
    oracle: CaptureManifest,
    candidate: CaptureManifest,
    bindings: dict[str, JsonValue],
) -> tuple[int, bool, set[Path]]:
    require_keys(values, {"packet", "decisions"}, "review")
    packet = resolve_artifact_binding(
        object_value(values, "packet"), evidence_root, "review.packet"
    )
    decision_records = object_list(values, "decisions", "review.decisions")
    decision_paths = [
        resolve_artifact_binding(record, evidence_root, "review.decision")
        for record in decision_records
    ]
    if len(set(decision_paths)) != len(decision_paths):
        raise MetricError("review.reviewer_set", "duplicated decision")
    packet_bindings = {
        key: value
        for key, value in bindings.items()
        if key not in {"command_plan", "command_plan_sha256"}
    }
    try:
        trusts, packet_hash = load_review_packet(
            packet, spec.pair_ids(), oracle, candidate, packet_bindings
        )
        reviews = []
        for path in decision_paths:
            raw = read_strict_object(path)
            trust = trusts.get(string_value(raw, "reviewer_id"))
            if trust is None:
                raise ReviewMaterializeError("review signer is not packet-bound")
            reviews.append(
                load_review_decision(path, spec.pair_ids(), packet_hash, trust)
            )
        if {review.reviewer_id for review in reviews} != set(trusts):
            raise ReviewMaterializeError("review signer set differs")
    except (ReviewMaterializeError, OSError, TypeError, ValueError) as error:
        raise MetricError("review.signature", "verification failed") from error
    all_passed = all(
        decision == "PASS"
        for review in reviews
        for decision, _critical in review.decisions.values()
    )
    return len(reviews), all_passed, {packet, *decision_paths}
