"""Bound Ed25519 executor for portable receipt requests."""

from __future__ import annotations

import argparse
import os
import stat
import sys
import tempfile
from collections.abc import Sequence
from pathlib import Path
from typing import Final

from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

from evaluate.multiformat_portable_receipt import (
    PortableReceiptInput,
    PortableReceiptVerification,
    sign_portable_receipt,
    verify_portable_receipt,
)
from evaluate.multiformat_portable_receipt_trust import load_portable_receipt_trust
from evaluate.multiformat_portable_receipt_validation import (
    object_array,
    require_exact_keys,
    validate_artifact_records,
)
from evaluate.multiformat_schema import (
    JsonValue,
    integer_value,
    sha256_value,
    string_value,
)
from evaluate.multiformat_strict_json import parse_strict_object_bytes

VERSION: Final = "multiformat-portable-receipt-executor 1"
MAX_REQUEST_BYTES: Final = 1024 * 1024
MAX_ARTIFACTS: Final = 4096
MAX_ARTIFACT_BYTES: Final = 2 * 1024 * 1024 * 1024
MAX_TOTAL_ARTIFACT_BYTES: Final = 16 * 1024 * 1024 * 1024


class PortableReceiptExecutorError(ValueError):
    """A bounded receipt request cannot be signed safely."""


def execute_receipt_request(
    request_path: Path,
    output_path: Path,
    lock_path: Path,
    evidence_root: Path,
    private_key_path: Path,
) -> Path:
    """Strict-parse, sign, verify, and exclusively publish one receipt."""
    temporary: Path | None = None
    try:
        root = evidence_root.resolve(strict=True)
        request = _load_request(request_path)
        destination = _evidence_path(root, output_path)
        trust = load_portable_receipt_trust(lock_path.resolve(strict=True), root)
        scope = sha256_value(request, "scope_sha256")
        if scope != trust.scope_sha256:
            raise PortableReceiptExecutorError("receipt request scope differs")
        nonce = sha256_value(request, "nonce")
        batch_id = string_value(request, "batch_id")
        if len(batch_id.encode("utf-8")) > 256:
            raise PortableReceiptExecutorError("receipt batch identity is too large")
        artifacts = object_array(request, "artifacts")
        _validate_caps(artifacts)
        private_key = _load_raw_private_key(private_key_path, root)
        if destination.exists():
            verified = verify_portable_receipt(
                destination, PortableReceiptVerification(trust)
            )
            _require_requested_identity(verified, nonce, batch_id, artifacts)
            return destination
        descriptor, name = tempfile.mkstemp(
            prefix=f".{destination.name}.", dir=destination.parent
        )
        os.close(descriptor)
        temporary = Path(name)
        temporary.unlink()
        sign_portable_receipt(
            temporary,
            PortableReceiptInput(trust, nonce, batch_id, artifacts),
            private_key,
        )
        verified = verify_portable_receipt(
            temporary,
            PortableReceiptVerification(
                trust,
                claim_replay=True,
                bound_receipt_path=destination,
            ),
        )
        _require_requested_identity(verified, nonce, batch_id, artifacts)
        try:
            os.link(temporary, destination)
        except FileExistsError:
            existing = verify_portable_receipt(
                destination, PortableReceiptVerification(trust)
            )
            _require_requested_identity(existing, nonce, batch_id, artifacts)
        return destination
    except PortableReceiptExecutorError:
        raise
    except (OSError, TypeError, UnicodeError, ValueError) as error:
        raise PortableReceiptExecutorError(
            "portable receipt execution failed"
        ) from error
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


def _load_request(path: Path) -> dict[str, JsonValue]:
    resolved = path.resolve(strict=True)
    size = resolved.stat().st_size
    if not 0 < size <= MAX_REQUEST_BYTES or not stat.S_ISREG(resolved.stat().st_mode):
        raise PortableReceiptExecutorError("receipt request exceeds its size cap")
    request = parse_strict_object_bytes(resolved.read_bytes())
    require_exact_keys(
        request,
        {"schema_version", "scope_sha256", "nonce", "batch_id", "artifacts"},
        "request",
    )
    if integer_value(request, "schema_version") != 1:
        raise PortableReceiptExecutorError("receipt request schema differs")
    return request


def _validate_caps(artifacts: list[dict[str, JsonValue]]) -> None:
    if len(artifacts) > MAX_ARTIFACTS:
        raise PortableReceiptExecutorError("receipt artifact count exceeds its cap")
    validate_artifact_records(artifacts)
    total = 0
    for artifact in artifacts:
        size = integer_value(artifact, "size")
        if size > MAX_ARTIFACT_BYTES:
            raise PortableReceiptExecutorError("receipt artifact exceeds its size cap")
        total += size
        if total > MAX_TOTAL_ARTIFACT_BYTES:
            raise PortableReceiptExecutorError(
                "receipt artifacts exceed their total cap"
            )
        if (
            len(string_value(artifact, "path").encode()) > 4096
            or len(string_value(artifact, "role").encode()) > 256
        ):
            raise PortableReceiptExecutorError("receipt artifact text exceeds its cap")


def _load_raw_private_key(path: Path, evidence_root: Path) -> Ed25519PrivateKey:
    if path.is_symlink():
        raise PortableReceiptExecutorError("receipt private key must not be a symlink")
    resolved = path.resolve(strict=True)
    if resolved.is_relative_to(evidence_root):
        raise PortableReceiptExecutorError("receipt private key enters evidence root")
    info = resolved.stat()
    if not stat.S_ISREG(info.st_mode) or stat.S_IMODE(info.st_mode) & 0o077:
        raise PortableReceiptExecutorError("receipt private key permissions differ")
    value = resolved.read_bytes()
    if len(value) != 32:
        raise PortableReceiptExecutorError("receipt private key size differs")
    return Ed25519PrivateKey.from_private_bytes(value)


def _evidence_path(root: Path, supplied: Path) -> Path:
    if supplied.is_symlink():
        raise PortableReceiptExecutorError("receipt output is symlinked")
    parent = supplied.parent.resolve(strict=True)
    destination = parent / supplied.name
    if not destination.is_relative_to(root) or destination == root:
        raise PortableReceiptExecutorError("receipt output escapes evidence root")
    if destination.exists() and not destination.is_file():
        raise PortableReceiptExecutorError("receipt output is not a file")
    return destination


def _require_requested_identity(
    identity: object,
    nonce: str,
    batch_id: str,
    artifacts: list[dict[str, JsonValue]],
) -> None:
    from evaluate.multiformat_portable_receipt import PortableReceiptIdentity
    from evaluate.multiformat_portable_receipt_replay import artifact_root_sha256

    if (
        not isinstance(identity, PortableReceiptIdentity)
        or identity.nonce != nonce
        or identity.batch_id != batch_id
        or identity.artifact_root_sha256 != artifact_root_sha256(artifacts)
    ):
        raise PortableReceiptExecutorError("signed receipt identity differs")


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Sign one strict portable receipt request."
    )
    parser.add_argument("--version", action="version", version=VERSION)
    parser.add_argument("--request", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--portable-lock", type=Path, required=True)
    parser.add_argument("--evidence-root", type=Path, required=True)
    parser.add_argument("--private-key", type=Path, required=True)
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    args = _parser().parse_args(argv)
    try:
        execute_receipt_request(
            args.request,
            args.output,
            args.portable_lock,
            args.evidence_root,
            args.private_key,
        )
    except (OSError, TypeError, ValueError):
        sys.stderr.write("portable receipt executor rejected the request\n")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
