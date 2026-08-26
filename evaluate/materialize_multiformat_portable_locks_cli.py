from __future__ import annotations

import argparse
import json
import sys
from collections.abc import Sequence
from pathlib import Path

from evaluate.materialize_multiformat_portable_locks import (
    PortableLockIncompleteError,
    PortableLockInputs,
    materialize_portable_locks,
)


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Materialize validated schema-2 portable reference locks and keys."
    )
    for name in (
        "project-root",
        "evidence-root",
        "output-dir",
        "contract",
        "evaluator",
        "libreoffice",
        "pdftoppm",
        "pdftotext",
        "pdfinfo",
        "canonicalizer",
        "font-bundle",
        "configuration",
        "chromium",
        "executor",
        "sandbox-exec",
        "browser-lock",
        "candidate-runtime-lock",
        "converter",
        "pdftohtml",
        "openssl",
        "receipt-signer",
        "candidate-sandbox-public-key",
        "private-key",
    ):
        parser.add_argument(f"--{name}", type=Path, required=True)
    parser.add_argument("--corpus-manifest", type=Path, action="append", required=True)
    parser.add_argument("--generate-keys", action="store_true")
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        values = vars(args)
        corpora = tuple(values.pop("corpus_manifest"))
        generate = bool(values.pop("generate_keys"))
        locks = materialize_portable_locks(
            PortableLockInputs(corpora=corpora, generate_keys=generate, **values)
        )
    except PortableLockIncompleteError as error:
        sys.stdout.write(
            json.dumps({"status": "INCOMPLETE", "reason": str(error)}, sort_keys=True)
            + "\n"
        )
        return 2
    except (OSError, TypeError, ValueError) as error:
        sys.stdout.write(
            json.dumps({"status": "FAIL", "reason": str(error)}, sort_keys=True) + "\n"
        )
        return 1
    sys.stdout.write(
        json.dumps(
            {"status": "READY", "locks": [path.as_posix() for path in locks]},
            sort_keys=True,
        )
        + "\n"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
