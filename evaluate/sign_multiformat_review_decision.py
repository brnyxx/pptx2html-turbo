from __future__ import annotations

import argparse
import json
import os
import stat
import sys
from collections.abc import Sequence
from pathlib import Path

from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

from evaluate.jcs import canonicalize
from evaluate.multiformat_corpus_items import require_keys
from evaluate.multiformat_schema import JsonValue, integer_value, string_value
from evaluate.multiformat_strict_json import read_strict_object


class ReviewSigningError(RuntimeError):
    pass


def _private_key(path: Path) -> Ed25519PrivateKey:
    try:
        before = path.lstat()
        if stat.S_ISLNK(before.st_mode) or not stat.S_ISREG(before.st_mode):
            raise ReviewSigningError("reviewer private key is not a regular file")
        descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
    except ReviewSigningError:
        raise
    except OSError as error:
        raise ReviewSigningError("reviewer private key is unavailable") from error
    try:
        current = os.fstat(descriptor)
        if not stat.S_ISREG(current.st_mode) or (current.st_dev, current.st_ino) != (
            before.st_dev,
            before.st_ino,
        ):
            raise ReviewSigningError("reviewer private key changed during open")
        if stat.S_IMODE(current.st_mode) & 0o077:
            raise ReviewSigningError("reviewer private key permissions are not private")
        with os.fdopen(descriptor, "rb", closefd=False) as stream:
            value = stream.read()
    finally:
        os.close(descriptor)
    if len(value) == 32:
        return Ed25519PrivateKey.from_private_bytes(value)
    loaded = serialization.load_pem_private_key(value, password=None)
    if not isinstance(loaded, Ed25519PrivateKey):
        raise ReviewSigningError("reviewer private key is not Ed25519")
    return loaded


def sign_review_decision(
    template: Path, private_key: Path, output: Path
) -> dict[str, JsonValue]:
    if output.exists():
        raise ReviewSigningError("signed review decision already exists")
    values = read_strict_object(template)
    require_keys(
        values,
        {
            "schema_version",
            "packet_sha256",
            "reviewer_id",
            "reviewer_role",
            "public_key_sha256",
            "checklist_version",
            "pairs",
        },
        "review.decision",
    )
    if (
        integer_value(values, "schema_version") != 2
        or string_value(values, "checklist_version") != "multiformat-review-v2"
    ):
        raise ReviewSigningError("unsupported review decision schema")
    key = _private_key(private_key)
    import hashlib

    public_digest = hashlib.sha256(key.public_key().public_bytes_raw()).hexdigest()
    if string_value(values, "public_key_sha256") != public_digest:
        raise ReviewSigningError("private key is not bound to this reviewer")
    payload = canonicalize(values)
    signed = {**values, "signature": key.sign(payload).hex()}
    encoded = canonicalize(signed)
    try:
        descriptor = os.open(output, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o644)
    except FileExistsError as error:
        raise ReviewSigningError("signed review decision already exists") from error
    with os.fdopen(descriptor, "wb") as stream:
        stream.write(encoded)
        stream.flush()
        os.fsync(stream.fileno())
    return {
        "status": "SIGNED",
        "decision": output.as_posix(),
        "reviewer_id": string_value(values, "reviewer_id"),
    }


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Create one packet-bound signed reviewer decision."
    )
    parser.add_argument("--decision", type=Path, required=True)
    parser.add_argument("--private-key", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        result = sign_review_decision(args.decision, args.private_key, args.output)
    except (OSError, ReviewSigningError, TypeError, ValueError) as error:
        sys.stdout.write(
            json.dumps({"status": "FAIL", "reason": str(error)}, sort_keys=True) + "\n"
        )
        return 1
    sys.stdout.write(json.dumps(result, sort_keys=True) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
