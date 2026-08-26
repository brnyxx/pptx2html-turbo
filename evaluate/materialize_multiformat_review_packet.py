from __future__ import annotations

import argparse
import json
import sys
from collections.abc import Sequence
from pathlib import Path

from evaluate.multiformat_metric_manifest import (
    MetricsAssemblyError,
    prepare_metric_context,
)
from evaluate.multiformat_review_materialize import ReviewMaterializeError
from evaluate.multiformat_review_packet import materialize_review_packet
from evaluate.multiformat_review_registry import ReviewRegistryError


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Validate captures and create an immutable blank review packet "
            "bound to the fixed reviewer registry."
        )
    )
    parser.add_argument("--project-root", type=Path, default=Path("."))
    parser.add_argument("--contract", type=Path, required=True)
    parser.add_argument("--corpus-manifest", type=Path, required=True)
    parser.add_argument("--evaluator-manifest", type=Path, required=True)
    parser.add_argument("--oracle-lock", type=Path, required=True)
    parser.add_argument("--oracle-capture", type=Path, required=True)
    parser.add_argument("--candidate-capture", type=Path, required=True)
    parser.add_argument("--evidence-root", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        context = prepare_metric_context(
            args.project_root.resolve(strict=True),
            args.contract,
            args.corpus_manifest,
            args.evaluator_manifest,
            args.oracle_lock,
            args.oracle_capture,
            args.candidate_capture,
            args.evidence_root.resolve(strict=True),
        )
        summary = materialize_review_packet(
            args.output_dir,
            context.oracle,
            context.candidate,
            context.spec.pair_ids(),
            bindings={
                "project_revision": context.project_revision,
                "contract_sha256": context.contract_hash,
                "corpus_manifest_sha256": context.corpus_hash,
                "evaluator_manifest_sha256": context.evaluator_hash,
                "oracle_lock_sha256": context.oracle_hash,
                "oracle_capture": context.oracle_binding,
                "candidate_capture": context.candidate_binding,
            },
        )
    except (
        MetricsAssemblyError,
        ReviewMaterializeError,
        ReviewRegistryError,
        OSError,
        TypeError,
        ValueError,
    ) as error:
        sys.stdout.write(
            json.dumps({"status": "FAIL", "reason": str(error)}, sort_keys=True) + "\n"
        )
        return 1
    sys.stdout.write(json.dumps(summary, sort_keys=True) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
