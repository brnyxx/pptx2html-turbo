from __future__ import annotations

import argparse
from collections.abc import Sequence
from pathlib import Path

from evaluate.multiformat_security_snapshot import (
    SecuritySnapshotError,
    generate_security_snapshot,
)
from evaluate.multiformat_security_snapshot_cli import emit_error, emit_summary


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Generate deterministic seven-format security sources.",
    )
    parser.add_argument("--contract", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    arguments = parser.parse_args(argv)
    try:
        summary = generate_security_snapshot(
            arguments.contract,
            arguments.output_dir,
        )
    except SecuritySnapshotError as error:
        emit_error(error)
        return 1
    emit_summary(summary)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
