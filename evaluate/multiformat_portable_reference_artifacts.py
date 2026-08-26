from __future__ import annotations

import os
import stat
from pathlib import Path

from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

from evaluate.multiformat_schema import JsonValue, sha256_file


def load_raw_private_key(path: Path) -> Ed25519PrivateKey:
    value = path.read_bytes()
    if len(value) != 32:
        raise ValueError("portable Ed25519 private key must contain exactly 32 bytes")
    mode = stat.S_IMODE(path.stat().st_mode)
    if mode & 0o077:
        raise ValueError("portable Ed25519 private key permissions must be 0600")
    return Ed25519PrivateKey.from_private_bytes(value)


def artifact_records(
    root: Path,
    values: list[tuple[Path, str]],
) -> list[dict[str, JsonValue]]:
    resolved_root = root.resolve(strict=True)
    records: list[dict[str, JsonValue]] = []
    seen: set[Path] = set()
    for path, role in values:
        resolved = path.resolve(strict=True)
        if resolved in seen or not resolved.is_relative_to(resolved_root):
            raise ValueError(
                "portable receipt artifact is duplicated or outside evidence root"
            )
        seen.add(resolved)
        records.append(
            {
                "path": resolved.relative_to(resolved_root).as_posix(),
                "sha256": sha256_file(resolved),
                "size": resolved.stat().st_size,
                "role": role,
            }
        )
    records.sort(key=lambda item: str(item["path"]))
    return records


def write_raw_keypair(private_path: Path, public_path: Path) -> None:
    if private_path.exists() or public_path.exists():
        raise FileExistsError("portable key destination already exists")
    private = Ed25519PrivateKey.generate()
    _exclusive_write(private_path, private.private_bytes_raw(), 0o600)
    try:
        _exclusive_write(public_path, private.public_key().public_bytes_raw(), 0o644)
    except Exception:
        private_path.unlink(missing_ok=True)
        raise


def _exclusive_write(path: Path, value: bytes, mode: int) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, mode)
    try:
        os.write(descriptor, value)
        os.fsync(descriptor)
    finally:
        os.close(descriptor)
