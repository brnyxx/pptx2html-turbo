from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Sequence

from evaluate.multiformat_candidate_capture import capture_candidate_evidence
from evaluate.multiformat_candidate_types import CandidateCaptureError
from evaluate.multiformat_metric_types import MetricError


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Capture network-isolated Chromium candidate evidence.",
    )
    parser.add_argument("--project-root", type=Path, default=Path("."))
    parser.add_argument("--contract", type=Path, required=True)
    parser.add_argument("--corpus-manifest", type=Path, required=True)
    parser.add_argument("--evaluator-manifest", type=Path, required=True)
    parser.add_argument("--oracle-lock", type=Path, required=True)
    parser.add_argument("--evidence-root", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--converter", type=Path, required=True)
    parser.add_argument("--soffice", type=Path, required=True)
    parser.add_argument("--pdftohtml", type=Path, required=True)
    parser.add_argument("--pdfinfo", type=Path, required=True)
    parser.add_argument("--chromium", type=Path, required=True)
    parser.add_argument("--font-bundle", type=Path, required=True)
    parser.add_argument("--sandbox-attestation", type=Path, required=True)
    parser.add_argument("--sandbox-public-key", type=Path, required=True)
    parser.add_argument("--openssl", type=Path, required=True)
    parser.add_argument("--receipt-signer", type=Path, required=True)
    parser.add_argument("--timeout-seconds", type=int, default=120)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        result = capture_candidate_evidence(
            args.project_root,
            args.contract,
            args.corpus_manifest,
            args.evaluator_manifest,
            args.oracle_lock,
            args.evidence_root,
            args.output_dir,
            converter=args.converter,
            soffice=args.soffice,
            pdftohtml=args.pdftohtml,
            pdfinfo=args.pdfinfo,
            chromium=args.chromium,
            font_bundle=args.font_bundle,
            sandbox_attestation=args.sandbox_attestation,
            sandbox_public_key=args.sandbox_public_key,
            openssl=args.openssl,
            receipt_signer=args.receipt_signer,
            timeout_seconds=args.timeout_seconds,
        )
    except (CandidateCaptureError, MetricError, OSError, ValueError) as error:
        sys.stdout.write(
            json.dumps(
                {"status": "FAIL", "reason": str(error)},
                ensure_ascii=True,
                sort_keys=True,
            )
            + "\n"
        )
        return 1
    sys.stdout.write(
        json.dumps(
            {
                "status": "READY",
                "capture_manifest": result.capture.as_posix(),
                "determinism": result.determinism.as_posix(),
            },
            ensure_ascii=True,
            sort_keys=True,
        )
        + "\n"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
