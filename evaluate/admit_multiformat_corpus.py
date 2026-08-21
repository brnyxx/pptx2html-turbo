# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
# ─── How to run ───
# uv run --python 3.11 python -m evaluate.admit_multiformat_corpus --help

from __future__ import annotations

import argparse
import json
import sys
from collections.abc import Sequence
from pathlib import Path

from evaluate.multiformat_corpus_admission import (
    AdmissionMetadata,
    AdmissionPlan,
    admit_corpus,
)
from evaluate.multiformat_corpus_identity import validate_admitted_corpus
from evaluate.multiformat_corpus_qualification import (
    QualificationCommands,
    qualification_validators,
)
from evaluate.multiformat_corpus_types import CorpusError, CorpusStatus


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Admit or validate a frozen seven-format corpus.",
    )
    subparsers = parser.add_subparsers(dest="command", required=True)
    validate = subparsers.add_parser("validate", help="validate published corpus")
    validate.add_argument("--contract", type=Path, required=True)
    validate.add_argument("--corpus-root", type=Path, required=True)
    admit = subparsers.add_parser(
        "admit", help="admit with real qualification evidence"
    )
    admit.add_argument("--contract", type=Path, required=True)
    admit.add_argument("--manifest", type=Path, action="append", required=True)
    admit.add_argument("--destination", type=Path, required=True)
    admit.add_argument("--corpus-revision", required=True)
    admit.add_argument("--project-revision", required=True)
    admit.add_argument("--admitted-at", required=True)
    admit.add_argument("--extraction-command", type=Path)
    admit.add_argument("--font-command", type=Path)
    admit.add_argument("--render-command", type=Path)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    if args.command == "validate":
        try:
            result = validate_admitted_corpus(args.contract, args.corpus_root)
        except CorpusError as error:
            _write({"status": "FAIL", "reason": error.reason, "detail": error.detail})
            return 1
        _write(
            {
                "status": result.status.value,
                "aggregate_sha256": result.aggregate_sha256,
                "source_count": result.source_count,
            }
        )
        return 0 if result.status is CorpusStatus.READY else 2
    command_paths = (
        args.extraction_command,
        args.font_command,
        args.render_command,
    )
    validators = None
    if all(path is not None for path in command_paths):
        try:
            validators = qualification_validators(
                QualificationCommands(
                    extraction=args.extraction_command,
                    fonts=args.font_command,
                    rendering=args.render_command,
                )
            )
        except CorpusError as error:
            _write({"status": "INCOMPLETE", "reasons": [error.reason]})
            return 2
    result = admit_corpus(
        AdmissionPlan(
            contract_path=args.contract,
            corpus_manifests=tuple(args.manifest),
            destination=args.destination,
            metadata=AdmissionMetadata(
                corpus_revision=args.corpus_revision,
                project_revision=args.project_revision,
                admitted_at=args.admitted_at,
            ),
        ),
        validators,
    )
    _write(
        {
            "status": result.status.value,
            "aggregate_sha256": result.aggregate_sha256,
            "reasons": list(result.reasons),
        }
    )
    return 0 if result.status is CorpusStatus.READY else 2


def _write(value: dict[str, str | int | list[str] | None]) -> None:
    sys.stdout.write(json.dumps(value, sort_keys=True) + "\n")


if __name__ == "__main__":
    raise SystemExit(main())
