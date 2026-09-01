from __future__ import annotations

import argparse
import sys
from collections.abc import Sequence
from pathlib import Path

from evaluate.multiformat_ready_assembly import assemble_ready_corpora
from evaluate.multiformat_ready_assembly_types import (
    ReadyAssemblyError,
    ReadyAssemblyInputs,
)
from evaluate.multiformat_ready_cli import (
    ReadySourceArguments,
    add_ready_source_arguments,
    emit_error,
    emit_summary,
    ready_input_paths,
)


class _Arguments(ReadySourceArguments):
    def __init__(self) -> None:
        super().__init__()
        self.output_dir = Path()


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Assemble the immutable seven-format READY corpus snapshot.",
    )
    add_ready_source_arguments(parser)
    _ = parser.add_argument("--output-dir", type=Path, required=True)
    arguments = parser.parse_args(argv, namespace=_Arguments())
    try:
        summary = assemble_ready_corpora(
            ReadyAssemblyInputs(ready_input_paths(arguments), arguments.output_dir)
        )
    except ReadyAssemblyError as error:
        emit_error(sys.stderr, error)
        return 1
    emit_summary(sys.stdout, summary)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
