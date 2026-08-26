from __future__ import annotations

import argparse
import json
import sys
from collections.abc import Sequence
from pathlib import Path

from evaluate.multiformat_corpus_items import object_list, require_keys
from evaluate.multiformat_review_materialize import (
    ReviewMaterializeError,
    load_review_decision,
)
from evaluate.multiformat_schema import integer_value, string_value
from evaluate.multiformat_strict_json import read_strict_object


def validate_completed_review(
    packet_path: Path, decision_path: Path
) -> dict[str, object]:
    packet = read_strict_object(packet_path)
    require_keys(
        packet,
        {"schema_version", "status", "checklist_version", "bindings", "pairs"},
        "review.packet",
    )
    if (
        integer_value(packet, "schema_version") != 1
        or string_value(packet, "status") != "INCOMPLETE"
        or string_value(packet, "checklist_version") != "multiformat-review-v1"
    ):
        raise ReviewMaterializeError("unsupported review packet schema")
    pair_ids: set[str] = set()
    for pair in object_list(packet, "pairs", "review.packet.pairs"):
        require_keys(
            pair,
            {
                "pair_id",
                "reference_png_sha256",
                "candidate_png_sha256",
                "reference_inventory_sha256",
                "candidate_inventory_sha256",
            },
            "review.packet.pair",
        )
        pair_id = string_value(pair, "pair_id")
        if pair_id in pair_ids:
            raise ReviewMaterializeError(f"duplicate review packet pair: {pair_id}")
        pair_ids.add(pair_id)
    decision = load_review_decision(decision_path, frozenset(pair_ids))
    return {
        "status": "VALID",
        "reviewer_id": decision.reviewer_id,
        "reviewer_role": decision.reviewer_role,
        "pair_count": len(decision.decisions),
    }


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Validate one completed review decision."
    )
    parser.add_argument("--review-packet", type=Path, required=True)
    parser.add_argument("--decision", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        summary = validate_completed_review(args.review_packet, args.decision)
    except (ReviewMaterializeError, OSError, TypeError, ValueError) as error:
        sys.stdout.write(
            json.dumps({"status": "FAIL", "reason": str(error)}, sort_keys=True) + "\n"
        )
        return 1
    sys.stdout.write(json.dumps(summary, sort_keys=True) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
