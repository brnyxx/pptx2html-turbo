from __future__ import annotations

import argparse
import json
import sys
from collections.abc import Sequence
from pathlib import Path
from typing import TextIO

from evaluate.multiformat_font_snapshot import (
    FontSnapshotError,
    FontSnapshotSummary,
    generate_font_snapshot,
    validate_font_snapshot,
)
from evaluate.multiformat_schema import JsonValue


def main(argv: Sequence[str] | None = None) -> int:
    """Run the font snapshot generation or validation subcommand."""
    parser = _parser()
    arguments = parser.parse_args(argv)
    try:
        summary = (
            generate_font_snapshot(tuple(arguments.font_dir), arguments.output_dir)
            if arguments.command == "generate"
            else validate_font_snapshot(arguments.manifest, arguments.snapshot_root)
        )
    except FontSnapshotError as error:
        _emit(
            sys.stderr,
            {"error": "font-snapshot", "message": str(error)},
        )
        return 1
    _emit(sys.stdout, _summary_value(summary))
    return 0


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="generate_multiformat_font_bundle")
    commands = parser.add_subparsers(dest="command", required=True)
    generate = commands.add_parser("generate")
    generate.add_argument("--font-dir", action="append", type=Path, required=True)
    generate.add_argument("--output-dir", type=Path, required=True)
    validate = commands.add_parser("validate")
    validate.add_argument("--manifest", type=Path, required=True)
    validate.add_argument("--snapshot-root", type=Path, required=True)
    return parser


def _summary_value(summary: FontSnapshotSummary) -> dict[str, JsonValue]:
    return {
        "environment_sha256": summary.environment_sha256,
        "files": summary.files,
        "fonts": summary.fonts,
        "manifest_sha256": summary.manifest_sha256,
    }


def _emit(stream: TextIO, value: dict[str, JsonValue]) -> None:
    stream.write(
        json.dumps(value, ensure_ascii=True, separators=(",", ":"), sort_keys=True)
        + "\n"
    )


if __name__ == "__main__":
    raise SystemExit(main())
