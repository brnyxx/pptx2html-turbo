from __future__ import annotations

import argparse
import sys
from collections.abc import Sequence
from pathlib import Path

from evaluate.multiformat_ready_assembly_types import (
    ReadyAssemblyError,
    ReadyValidationInputs,
)
from evaluate.multiformat_ready_cli import (
    ReadySourceArguments,
    add_ready_source_arguments,
    emit_error,
    emit_summary,
    ready_input_paths,
)
from evaluate.multiformat_ready_validation import validate_ready_corpora


class _Arguments(ReadySourceArguments):
    def __init__(self) -> None:
        super().__init__()
        self.corpus_root = Path()


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Independently validate a seven-format READY corpus snapshot.",
    )
    add_ready_source_arguments(parser)
    _ = parser.add_argument("--corpus-root", type=Path, required=True)
    arguments = parser.parse_args(argv, namespace=_Arguments())
    try:
        summary = validate_ready_corpora(
            ReadyValidationInputs(ready_input_paths(arguments), arguments.corpus_root)
        )
    except ReadyAssemblyError as error:
        emit_error(sys.stderr, error)
        return 1
    emit_summary(sys.stdout, summary)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
