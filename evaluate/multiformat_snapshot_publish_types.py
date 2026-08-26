"""Identity and teardown records for atomic snapshot publishing.

Extracted from `multiformat_snapshot_publish` so that module holds only the
publication sequence. The teardown behaviour itself stays there, because its
filesystem seams are patched by the race tests.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True, slots=True)
class Identity:
    """Device and inode of a path, captured before it is trusted again."""

    device: int
    inode: int


@dataclass(frozen=True, slots=True)
class CleanupState:
    """Everything teardown must release after a publication attempt."""

    parent_descriptor: int
    staging: Path | None
    target: Path
    staging_descriptor: int | None
    staging_identity: Identity | None
    renamed: bool
    lock_descriptor: int
    lock: Path
    lock_identity: Identity
    published: bool


class InvalidLockNamespaceError(ValueError):
    __slots__ = ("namespace",)

    def __init__(self, namespace: str) -> None:
        self.namespace = namespace
        super().__init__(namespace)

    def __str__(self) -> str:
        return "lock_namespace must be a lowercase ASCII token"
