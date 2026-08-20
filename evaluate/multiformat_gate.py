from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Sequence

from evaluate.multiformat_contract import (
    FormatGateResult,
    GateStatus,
    GateSummary,
    evaluate_reports,
)

__all__ = [
    "FormatGateResult",
    "GateStatus",
    "GateSummary",
    "evaluate_reports",
    "main",
]


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Validate all seven document-format acceptance reports.",
    )
    parser.add_argument(
        "--contract",
        type=Path,
        default=Path(__file__).parent / "multiformat" / "contract.v1.json",
    )
    parser.add_argument("--reports-dir", type=Path, required=True)
    parser.add_argument("--oracle-lock", type=Path, required=True)
    parser.add_argument(
        "--evidence-root",
        type=Path,
        help="Root for evidence paths in reports (default: reports parent)",
    )
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    summary = evaluate_reports(
        args.contract,
        args.reports_dir,
        args.oracle_lock,
        args.evidence_root,
    )
    sys.stdout.write(
        json.dumps(summary.to_json_value(), ensure_ascii=True, sort_keys=True) + "\n",
    )
    if summary.status is GateStatus.PASS:
        return 0
    if summary.status is GateStatus.INCOMPLETE:
        return 2
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
