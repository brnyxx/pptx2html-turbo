from __future__ import annotations

import base64
import shutil
import stat
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

from evaluate.multiformat_candidate_artifacts import write_canonical_json
from evaluate.multiformat_candidate_attestation import canonical_payload
from evaluate.multiformat_schema import JsonValue, sha256_file


class AttestationFixtureError(Exception):
    pass


@dataclass(frozen=True, slots=True)
class TestVerifier:
    private_key: Path
    public_key: Path
    openssl: Path


def create_test_verifier(root: Path) -> TestVerifier:
    private_key = root / "test-sandbox-private.pem"
    public_key = root / "test-sandbox-public.pem"
    openssl_value = shutil.which("openssl")
    if openssl_value is None:
        raise AttestationFixtureError("OpenSSL is required for test attestations")
    openssl = Path(openssl_value).resolve(strict=True)
    if private_key.exists() and public_key.exists():
        return TestVerifier(private_key, public_key, openssl)
    subprocess.run(
        [
            openssl.as_posix(),
            "genpkey",
            "-algorithm",
            "ED25519",
            "-out",
            private_key.as_posix(),
        ],
        check=True,
        capture_output=True,
    )
    subprocess.run(
        [
            openssl.as_posix(),
            "pkey",
            "-in",
            private_key.as_posix(),
            "-pubout",
            "-out",
            public_key.as_posix(),
        ],
        check=True,
        capture_output=True,
    )
    return TestVerifier(private_key, public_key, openssl)


def verifier_lock(verifier: TestVerifier) -> dict[str, JsonValue]:
    return {
        "algorithm": "ed25519",
        "verifier_id": "test-verifier",
        "public_key_sha256": sha256_file(verifier.public_key),
        "openssl_sha256": sha256_file(verifier.openssl),
    }


def write_signed_attestation(
    path: Path,
    verifier: TestVerifier,
    payload: dict[str, JsonValue],
) -> None:
    payload_path = path.with_suffix(".payload")
    signature_path = path.with_suffix(".signature")
    payload_path.write_bytes(canonical_payload(payload))
    subprocess.run(
        [
            verifier.openssl.as_posix(),
            "pkeyutl",
            "-sign",
            "-inkey",
            verifier.private_key.as_posix(),
            "-rawin",
            "-in",
            payload_path.as_posix(),
            "-out",
            signature_path.as_posix(),
        ],
        check=True,
        capture_output=True,
    )
    write_canonical_json(
        path,
        {
            **payload,
            "signature": base64.b64encode(signature_path.read_bytes()).decode(),
        },
    )


def write_receipt_signer(root: Path, verifier: TestVerifier) -> Path:
    path = root / "receipt-signer.py"
    script = (
        f"#!{sys.executable}\n"
        "import base64,json,pathlib,subprocess,sys,tempfile\n"
        f"OPENSSL={verifier.openssl.as_posix()!r}\n"
        f"PRIVATE_KEY={verifier.private_key.as_posix()!r}\n"
        "args=sys.argv[1:]\n"
        "if '--version' in args:\n"
        " print('receipt-signer test-version'); raise SystemExit(0)\n"
        "request=pathlib.Path(args[args.index('--request')+1])\n"
        "output=pathlib.Path(args[args.index('--output')+1])\n"
        "payload=json.loads(request.read_text())\n"
        "data=json.dumps(payload,ensure_ascii=True,sort_keys=True,separators=(',',':')).encode()\n"
        "with tempfile.TemporaryDirectory() as d:\n"
        " p=pathlib.Path(d); raw=p/'payload'; sig=p/'signature'; raw.write_bytes(data)\n"
        " subprocess.run([OPENSSL,'pkeyutl','-sign','-inkey',PRIVATE_KEY,'-rawin','-in',str(raw),'-out',str(sig)],check=True)\n"
        " payload['signature']=base64.b64encode(sig.read_bytes()).decode()\n"
        "output.write_text(json.dumps(payload,ensure_ascii=True,indent=2,sort_keys=True)+'\\n')\n"
    )
    path.write_text(script, encoding="utf-8")
    path.chmod(path.stat().st_mode | stat.S_IXUSR)
    return path
