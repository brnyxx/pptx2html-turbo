from __future__ import annotations

import shutil
from pathlib import Path

from evaluate.multiformat_candidate_artifacts import write_canonical_json
from evaluate.multiformat_capture_types import CaptureManifest
from evaluate.multiformat_corpus_items import canonical_identity
from evaluate.multiformat_review_materialize import (
    ReviewMaterializeError,
    review_pair_artifacts,
)
from evaluate.multiformat_schema import JsonValue, sha256_file


def materialize_review_packet(
    output_dir: Path,
    oracle: CaptureManifest,
    candidate: CaptureManifest,
    expected_pairs: frozenset[str],
    *,
    reviewer_id_1: str,
    reviewer_role_1: str,
    reviewer_id_2: str,
    reviewer_role_2: str,
    bindings: dict[str, JsonValue],
) -> dict[str, JsonValue]:
    reviewers = (
        (
            canonical_identity(reviewer_id_1, "reviewer_id"),
            canonical_identity(reviewer_role_1, "reviewer_role"),
        ),
        (
            canonical_identity(reviewer_id_2, "reviewer_id"),
            canonical_identity(reviewer_role_2, "reviewer_role"),
        ),
    )
    if (
        len({item[0] for item in reviewers}) != 2
        or len({item[1] for item in reviewers}) != 2
    ):
        raise ReviewMaterializeError("reviewer identities and roles must be distinct")
    if not expected_pairs:
        raise ReviewMaterializeError("review packet pair set is empty")
    output_dir.mkdir(parents=True, exist_ok=False)
    try:
        packet_path = output_dir / "review-packet.json"
        write_canonical_json(
            packet_path,
            {
                "schema_version": 1,
                "status": "INCOMPLETE",
                "checklist_version": "multiformat-review-v1",
                "bindings": bindings,
                "pairs": [
                    review_pair_artifacts(pair_id, oracle, candidate)
                    for pair_id in sorted(expected_pairs)
                ],
            },
        )
        templates: list[JsonValue] = []
        for reviewer_id, reviewer_role in reviewers:
            path = output_dir / f"decision-{reviewer_id}.json"
            write_canonical_json(
                path,
                {
                    "schema_version": 1,
                    "reviewer_id": reviewer_id,
                    "reviewer_role": reviewer_role,
                    "independent": False,
                    "checklist_version": "multiformat-review-v1",
                    "pairs": [
                        {
                            "pair_id": pair_id,
                            "decision": None,
                            "critical_defect": None,
                        }
                        for pair_id in sorted(expected_pairs)
                    ],
                },
            )
            templates.append(path.as_posix())
        return {
            "status": "INCOMPLETE",
            "review_packet": packet_path.as_posix(),
            "review_packet_sha256": sha256_file(packet_path),
            "decision_templates": templates,
        }
    except BaseException:
        shutil.rmtree(output_dir)
        raise


__all__ = ["materialize_review_packet"]
