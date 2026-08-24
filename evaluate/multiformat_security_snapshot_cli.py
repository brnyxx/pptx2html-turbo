from __future__ import annotations

import json
import sys
from typing import TextIO

from evaluate.multiformat_schema import JsonValue
from evaluate.multiformat_security_snapshot import (
    SecuritySnapshotError,
    SecuritySnapshotSummary,
)


def emit_summary(summary: SecuritySnapshotSummary) -> None:
    counts: dict[str, JsonValue] = {
        name: count for name, count in summary.counts.items()
    }
    _emit(
        sys.stdout,
        {
            "counts": counts,
            "files": summary.files,
            "manifest_sha256": summary.manifest_sha256,
            "schema_version": 1,
            "status": summary.status,
        },
    )


def emit_error(error: SecuritySnapshotError) -> None:
    _emit(
        sys.stderr,
        {
            "error": "security-snapshot",
            "message": str(error),
        },
    )


def _emit(stream: TextIO, value: dict[str, JsonValue]) -> None:
    stream.write(
        json.dumps(
            value,
            ensure_ascii=True,
            separators=(",", ":"),
            sort_keys=True,
        )
        + "\n"
    )
