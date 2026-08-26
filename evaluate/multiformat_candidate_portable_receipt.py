from __future__ import annotations

from collections.abc import Callable
from pathlib import Path

from evaluate.multiformat_candidate_artifacts import write_canonical_json
from evaluate.multiformat_candidate_process import run_bounded_process
from evaluate.multiformat_candidate_types import CandidateCaptureError
from evaluate.multiformat_portable_receipt import (
    PortableReceiptIdentity,
    PortableReceiptVerification,
    verify_portable_receipt,
)
from evaluate.multiformat_portable_receipt_trust import load_portable_receipt_trust
from evaluate.multiformat_schema import JsonValue, sha256_file
from evaluate.multiformat_subprocess import clean_subprocess_environment


class CandidatePortableReceiptError(CandidateCaptureError):
    pass


_REQUIRED_CAPTURE_ROLES = frozenset(
    {
        "capture-runtime-identity",
        "capture-execution-log",
        "capture-unit-png",
        "capture-unit-inventory",
        "capture-candidate-html",
        "capture-candidate-determinism",
    }
)

_OPTIONAL_CAPTURE_ROLES = frozenset(
    {"capture-candidate-inventory-manifest", "security-execution"}
)


ReceiptExecutor = Callable[[Path, Path, Path], None]


def write_portable_candidate_receipt(
    evidence_root: Path,
    output_dir: Path,
    lock_path: Path,
    executor: Path,
    *,
    batch_id: str,
    artifacts: dict[Path, str],
    execute: ReceiptExecutor | None = None,
    require_capture_roles: bool = False,
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
            "schema_version": 2,
            "scope_sha256": trust.scope_sha256,
            "batch_id": batch_id,
            "artifacts": list(records),
        }
        write_canonical_json(request, request_value)
        (execute or _execute)(executor, request, receipt)
        verified = verify_portable_receipt(
            receipt,
            PortableReceiptVerification(trust=trust),
        )
        if verified.scope_sha256 != trust.scope_sha256:
            raise CandidatePortableReceiptError("portable receipt identity differs")
        if require_capture_roles:
            validate_candidate_capture_roles(verified)
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


def validate_candidate_capture_roles(identity: PortableReceiptIdentity) -> None:
    roles = {artifact.role for artifact in identity.artifacts}
    missing = _REQUIRED_CAPTURE_ROLES - roles
    if missing:
        raise CandidatePortableReceiptError(
            f"portable candidate capture roles differ: {min(missing)}"
        )

    unexpected = roles - _REQUIRED_CAPTURE_ROLES - _OPTIONAL_CAPTURE_ROLES
    if unexpected:
        raise CandidatePortableReceiptError(
            f"portable candidate capture role is unsupported: {min(unexpected)}"
        )


def _execute(executor: Path, request: Path, output: Path) -> None:
    command = (
        executor.resolve(strict=True).as_posix(),
        "--request",
        request.as_posix(),
        "--output",
        output.as_posix(),
    )
    exit_code = run_bounded_process(
        command,
        output.parent,
        clean_subprocess_environment(),
        output.parent / "receipt-executor.stdout.log",
        output.parent / "receipt-executor.stderr.log",
        timeout_seconds=30,
        max_log_bytes=1024 * 1024,
    )
    if exit_code != 0:
        raise CandidatePortableReceiptError(
            "portable receipt executor rejected request"
        )


__all__ = [
    "CandidatePortableReceiptError",
    "ReceiptExecutor",
    "validate_candidate_capture_roles",
    "write_portable_candidate_receipt",
]
