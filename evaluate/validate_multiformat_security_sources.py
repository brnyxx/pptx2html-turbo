from __future__ import annotations

import argparse
from collections.abc import Sequence
from pathlib import Path

from evaluate.multiformat_security_snapshot import SecuritySnapshotError
from evaluate.multiformat_security_snapshot_cli import emit_error, emit_summary
from evaluate.multiformat_security_snapshot_validation import (
    validate_security_snapshot,
)


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Validate deterministic seven-format security sources.",
    )
    parser.add_argument("--contract", type=Path, required=True)
    parser.add_argument("--manifest", type=Path, required=True)
    arguments = parser.parse_args(argv)
    try:
        summary = validate_security_snapshot(
            arguments.contract,
            arguments.manifest,
        )
    except SecuritySnapshotError as error:
        emit_error(error)
        return 1
    emit_summary(summary)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
