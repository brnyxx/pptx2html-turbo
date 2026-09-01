from __future__ import annotations

import base64
import subprocess
import tempfile
from pathlib import Path

from evaluate.multiformat_candidate_types import CandidateCaptureError
from evaluate.multiformat_schema import (
    JsonValue,
    sha256_file,
    sha256_value,
    string_value,
)
from evaluate.multiformat_subprocess import clean_subprocess_environment


def verify_ed25519_json(
    payload: bytes,
    signature_value: str,
    public_key_path: Path,
    openssl_path: Path,
    verifier: dict[str, JsonValue],
    label: str,
) -> None:
    public_key = public_key_path.resolve(strict=True)
    openssl = openssl_path.resolve(strict=True)
    if (
        string_value(verifier, "algorithm") != "ed25519"
        or sha256_file(public_key) != sha256_value(verifier, "public_key_sha256")
        or sha256_file(openssl) != sha256_value(verifier, "openssl_sha256")
    ):
        raise CandidateCaptureError(f"{label} lock mismatch")
    try:
        signature = base64.b64decode(signature_value, validate=True)
    except ValueError as error:
        raise CandidateCaptureError(f"{label} signature is invalid") from error
    with tempfile.TemporaryDirectory(prefix="candidate-signature-") as temp_dir:
        root = Path(temp_dir)
        payload_path = root / "payload.json"
        signature_path = root / "signature.bin"
        payload_path.write_bytes(payload)
        signature_path.write_bytes(signature)
        result = subprocess.run(
            [
                openssl.as_posix(),
                "pkeyutl",
                "-verify",
                "-pubin",
                "-inkey",
                public_key.as_posix(),
                "-rawin",
                "-in",
                payload_path.as_posix(),
                "-sigfile",
                signature_path.as_posix(),
            ],
            check=False,
            capture_output=True,
            env=clean_subprocess_environment(),
            timeout=15,
        )
    if result.returncode != 0:
        raise CandidateCaptureError(f"{label} signature verification failed")


__all__ = ["verify_ed25519_json"]
