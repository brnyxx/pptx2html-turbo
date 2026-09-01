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
from evaluate.multiformat_gate_types import FormatOracleLock, OracleLockInput

__all__ = [
    "FormatGateResult",
    "GateStatus",
    "GateSummary",
    "OracleLockInput",
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
    locks = parser.add_mutually_exclusive_group(required=True)
    locks.add_argument(
        "--oracle-lock",
        action="append",
        metavar="PATH|FORMAT=PATH",
        help=(
            "one schema-1 shared PATH, or repeat FORMAT=PATH once for each "
            "required format using schema-2 locks"
        ),
    )
    locks.add_argument(
        "--oracle-lock-dir",
        type=Path,
        help="directory containing one schema-2 {format}.json lock per format",
    )
    parser.add_argument(
        "--evidence-root",
        type=Path,
        help="Root for evidence paths in reports (default: reports parent)",
    )
    return parser.parse_args(argv)


def _parse_oracle_locks(
    values: list[str] | None,
    directory: Path | None,
) -> OracleLockInput:
    if directory is not None:
        return OracleLockInput.lock_directory(directory)
    if not values:
        raise ValueError("an oracle lock input is required")
    if len(values) == 1 and "=" not in values[0]:
        return OracleLockInput(shared=Path(values[0]))
    entries: list[FormatOracleLock] = []
    for value in values:
        document_format, separator, raw_path = value.partition("=")
        if not separator or not document_format or not raw_path:
            raise ValueError("per-format oracle locks must use FORMAT=PATH")
        entries.append(FormatOracleLock(document_format, Path(raw_path)))
    return OracleLockInput(format_locks=tuple(entries))


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        oracle_locks = _parse_oracle_locks(args.oracle_lock, args.oracle_lock_dir)
    except ValueError as error:
        sys.stderr.write(f"error: {error}\n")
        return 2
    summary = evaluate_reports(
        args.contract,
        args.reports_dir,
        oracle_locks,
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
