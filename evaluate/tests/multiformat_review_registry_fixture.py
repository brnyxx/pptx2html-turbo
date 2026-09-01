"""Temporary reviewer registries for unit tests.

Test keypairs are generated into a temporary directory at run time. No test
private key is ever committed, and this fixture is only reachable from tests:
production always resolves the tracked registry through
``multiformat_review_registry.REGISTRY_PATH``.
"""

from __future__ import annotations

import hashlib
import json
import os
from dataclasses import dataclass
from pathlib import Path

from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

from evaluate.multiformat_review_registry import (
    ReviewerRegistry,
    load_reviewer_registry,
)

TEST_REVIEWERS: tuple[tuple[str, str, str], ...] = (
    ("visual", "test-visual-reviewer", "visual-fidelity"),
    ("semantic", "test-semantic-reviewer", "semantic-security"),
)


@dataclass(frozen=True, slots=True)
class TestRegistry:
    path: Path
    private_keys: dict[str, Path]

    def private_key(self, reviewer_id: str) -> Path:
        return self.private_keys[reviewer_id]

    def load(self) -> ReviewerRegistry:
        """Loads this registry, raising for the intentionally invalid cases."""
        return load_reviewer_registry(self.path)


def write_test_registry(
    root: Path,
    *,
    reviewers: tuple[tuple[str, str, str], ...] = TEST_REVIEWERS,
    duplicate_key: bool = False,
    seeds: tuple[bytes, ...] | None = None,
) -> TestRegistry:
    """Writes a registry plus throwaway keypairs beneath ``root``."""
    keys_dir = root / "reviewer-keys"
    keys_dir.mkdir(parents=True, exist_ok=True)
    entries: list[dict[str, object]] = []
    private_keys: dict[str, Path] = {}
    shared: bytes | None = None
    for index, (slot, reviewer_id, role) in enumerate(reviewers):
        if seeds is not None:
            key = Ed25519PrivateKey.from_private_bytes(seeds[index])
        else:
            key = Ed25519PrivateKey.generate()
        private_path = root / f"{slot}.private"
        private_path.write_bytes(key.private_bytes_raw())
        os.chmod(private_path, 0o600)
        private_keys[reviewer_id] = private_path
        public = key.public_key().public_bytes_raw()
        if duplicate_key:
            shared = shared or public
            public = shared
        public_path = keys_dir / f"{slot}.public"
        public_path.write_bytes(public)
        os.chmod(public_path, 0o644)
        entries.append(
            {
                "reviewer_id": reviewer_id,
                "reviewer_role": role,
                "algorithm": "ed25519",
                "public_key_path": f"reviewer-keys/{slot}.public",
                "public_key_sha256": hashlib.sha256(public).hexdigest(),
            }
        )
    path = root / "reviewer-registry.v1.json"
    path.write_text(
        json.dumps(
            {"schema_version": 1, "algorithm": "ed25519", "reviewers": entries},
            sort_keys=True,
        ),
        encoding="utf-8",
    )
    return TestRegistry(path, private_keys)
