from __future__ import annotations

import argparse
import json
import sys
from collections.abc import Sequence
from pathlib import Path

from evaluate.multiformat_portable_reference_materializer import (
    PortableReferenceMaterializeError,
    materialize_portable_references,
)


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Materialize signed portable LibreOffice/Poppler references."
    )
    parser.add_argument("--contract", type=Path, required=True)
    parser.add_argument("--corpus-manifest", type=Path, required=True)
    parser.add_argument("--portable-lock", type=Path, required=True)
    parser.add_argument("--evidence-root", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--private-key", type=Path, required=True)
    parser.add_argument("--run-nonce", required=True)
    parser.add_argument("--batch-id", required=True)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        capture = materialize_portable_references(
            args.contract,
            args.corpus_manifest,
            args.portable_lock,
            args.evidence_root,
            args.output_dir,
            args.private_key,
            nonce=args.run_nonce,
            batch_id=args.batch_id,
        )
    except (PortableReferenceMaterializeError, OSError, ValueError) as error:
        sys.stdout.write(
            json.dumps({"status": "FAIL", "reason": str(error)}, sort_keys=True) + "\n"
        )
        return 1
    sys.stdout.write(
        json.dumps(
            {"status": "READY", "capture_manifest": capture.as_posix()}, sort_keys=True
        )
        + "\n"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
