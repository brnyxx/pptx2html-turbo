from __future__ import annotations

import argparse
import os
import sys
from collections.abc import Sequence
from pathlib import Path


class OracleSentinelCreateError(ValueError):
    """The oracle denial sentinel cannot be published safely."""


def create_oracle_sentinel(
    evidence_root: Path,
    oracle_root: Path,
    sentinel: Path,
    document_format: str,
) -> Path:
    root = evidence_root.resolve(strict=True)
    unresolved_oracle = oracle_root.resolve(strict=False)
    if not unresolved_oracle.is_relative_to(root) or unresolved_oracle == root:
        raise OracleSentinelCreateError("oracle root escapes evidence root")
    unresolved_oracle.mkdir(parents=True, exist_ok=True)
    resolved_oracle = unresolved_oracle.resolve(strict=True)
    if not resolved_oracle.is_dir() or not resolved_oracle.is_relative_to(root):
        raise OracleSentinelCreateError("oracle root escapes evidence root")
    destination = sentinel.resolve(strict=False)
    if destination.parent.resolve(strict=True) != resolved_oracle:
        raise OracleSentinelCreateError("oracle sentinel escapes oracle root")
    payload = f"candidate oracle denial sentinel: {document_format}\n".encode()
    try:
        descriptor = os.open(
            destination,
            os.O_WRONLY | os.O_CREAT | os.O_EXCL,
            0o600,
        )
    except FileExistsError as error:
        raise OracleSentinelCreateError("oracle sentinel already exists") from error
    try:
        os.write(descriptor, payload)
        os.fsync(descriptor)
    finally:
        os.close(descriptor)
    return destination


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Create one oracle denial sentinel.")
    parser.add_argument("--evidence-root", type=Path, required=True)
    parser.add_argument("--oracle-root", type=Path, required=True)
    parser.add_argument("--sentinel", type=Path, required=True)
    parser.add_argument("--format", required=True)
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    args = _parser().parse_args(argv)
    try:
        create_oracle_sentinel(
            args.evidence_root,
            args.oracle_root,
            args.sentinel,
            args.format,
        )
    except (OSError, TypeError, ValueError):
        sys.stderr.write("oracle sentinel creation failed\n")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
