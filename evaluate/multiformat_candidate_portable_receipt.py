from __future__ import annotations

import subprocess
from collections.abc import Callable
from pathlib import Path

from evaluate.multiformat_candidate_artifacts import write_canonical_json
from evaluate.multiformat_candidate_types import CandidateCaptureError
from evaluate.multiformat_portable_receipt import (
    PortableReceiptVerification,
    verify_portable_receipt,
)
from evaluate.multiformat_portable_receipt_trust import load_portable_receipt_trust
from evaluate.multiformat_schema import JsonValue, sha256_file
from evaluate.multiformat_subprocess import clean_subprocess_environment


class CandidatePortableReceiptError(CandidateCaptureError):
    pass


ReceiptExecutor = Callable[[Path, Path, Path], None]


def write_portable_candidate_receipt(
    evidence_root: Path,
    output_dir: Path,
    lock_path: Path,
    executor: Path,
    *,
    nonce: str,
    batch_id: str,
    artifacts: dict[Path, str],
    execute: ReceiptExecutor | None = None,
) -> Path:
    """Request and verify a host-signed portable candidate receipt."""
    try:
        trust = load_portable_receipt_trust(lock_path, evidence_root)
        records = [
            _record(evidence_root, path, role) for path, role in artifacts.items()
        ]
        records.sort(key=lambda value: str(value["path"]))
        request = output_dir / "portable-receipt-request.json"
        receipt = output_dir / "portable-execution-receipt.json"
        request_value: dict[str, JsonValue] = {
            "schema_version": 1,
            "scope_sha256": trust.scope_sha256,
            "nonce": nonce,
            "batch_id": batch_id,
            "artifacts": list(records),
        }
        write_canonical_json(request, request_value)
        (execute or _execute)(executor, request, receipt)
        verified = verify_portable_receipt(
            receipt,
            PortableReceiptVerification(trust=trust),
        )
        if verified.nonce != nonce or verified.scope_sha256 != trust.scope_sha256:
            raise CandidatePortableReceiptError("portable receipt identity differs")
        return receipt
    except CandidatePortableReceiptError:
        raise
    except (OSError, TypeError, ValueError) as error:
        raise CandidatePortableReceiptError(
            "portable candidate receipt failed"
        ) from error


def _record(root: Path, path: Path, role: str) -> dict[str, JsonValue]:
    resolved = path.resolve(strict=True)
    return {
        "path": resolved.relative_to(root.resolve(strict=True)).as_posix(),
        "sha256": sha256_file(resolved),
        "size": resolved.stat().st_size,
        "role": role,
    }


def _execute(executor: Path, request: Path, output: Path) -> None:
    result = subprocess.run(
        [
            executor.resolve(strict=True).as_posix(),
            "--request",
            request.as_posix(),
            "--output",
            output.as_posix(),
        ],
        check=False,
        capture_output=True,
        env=clean_subprocess_environment(),
        timeout=30,
    )
    if (
        result.returncode != 0
        or len(result.stdout) > 1024 * 1024
        or len(result.stderr) > 1024 * 1024
    ):
        raise CandidatePortableReceiptError(
            "portable receipt executor rejected request"
        )


__all__ = [
    "CandidatePortableReceiptError",
    "ReceiptExecutor",
    "write_portable_candidate_receipt",
]
