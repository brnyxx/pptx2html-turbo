from __future__ import annotations

from pathlib import Path

from evaluate.multiformat_portable_lock_io import exclusive_write
from evaluate.multiformat_portable_reference_artifacts import (
    load_raw_private_key,
    write_raw_keypair,
)


def prepare_key_material(
    project_root: Path,
    evidence_root: Path,
    output_dir: Path,
    private_key: Path,
    generate: bool,
) -> tuple[Path, Path, Path]:
    root = evidence_root.resolve(strict=True)
    output = output_dir.parent.resolve(strict=True) / output_dir.name
    if not output.is_relative_to(root):
        raise ValueError("portable lock output must be inside evidence root")
    if output.exists():
        raise ValueError("portable lock output already exists")
    destination = private_key.resolve(strict=False)
    if destination.is_relative_to(
        project_root.resolve(strict=True)
    ) or destination.is_relative_to(root):
        raise ValueError(
            "portable private key must remain outside the project and evidence root"
        )
    public = output / "keys/public.raw"
    if generate:
        write_raw_keypair(private_key, public)
        load_raw_private_key(private_key)
    else:
        private = load_raw_private_key(private_key)
        public.parent.mkdir(parents=True)
        exclusive_write(public, private.public_key().public_bytes_raw(), 0o644)
    return root, output, public.resolve(strict=True)
