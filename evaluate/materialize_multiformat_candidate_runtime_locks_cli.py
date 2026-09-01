from __future__ import annotations

import argparse
import json
import sys
from collections.abc import Sequence
from pathlib import Path

from evaluate.materialize_multiformat_candidate_runtime_locks import (
    CandidateRuntimeLockIncompleteError,
    CandidateRuntimeLockInputs,
    materialize_candidate_runtime_locks,
)
from evaluate.multiformat_candidate_types import CandidateCaptureError


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Write candidate browser and schema-1 runtime locks "
            "for schema-2 outer locks."
        )
    )
    for name in (
        "project-root",
        "evidence-root",
        "output-dir",
        "converter",
        "soffice",
        "pdftohtml",
        "pdfinfo",
        "receipt-signer",
        "chromium",
        "font-bundle",
        "sandbox-public-key",
        "openssl",
    ):
        parser.add_argument(f"--{name}", type=Path, required=True)
    parser.add_argument("--verifier-id", required=True)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    try:
        browser, runtime = materialize_candidate_runtime_locks(
            CandidateRuntimeLockInputs(**vars(arguments))
        )
    except CandidateRuntimeLockIncompleteError as error:
        status, reason, code = "INCOMPLETE", str(error), 2
    except (CandidateCaptureError, OSError, TypeError, ValueError) as error:
        status, reason, code = "FAIL", str(error), 1
    else:
        sys.stdout.write(
            json.dumps(
                {
                    "status": "READY",
                    "browser_lock": browser.as_posix(),
                    "candidate_runtime_lock": runtime.as_posix(),
                },
                sort_keys=True,
            )
            + "\n"
        )
        return 0
    sys.stdout.write(
        json.dumps({"status": status, "reason": reason}, sort_keys=True) + "\n"
    )
    return code


if __name__ == "__main__":
    raise SystemExit(main())
