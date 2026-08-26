"""Canonical identity for an immutable READY corpus tree."""

from __future__ import annotations

import hashlib
from dataclasses import dataclass
from pathlib import Path
from typing import Final

from evaluate.jcs import JcsError, canonicalize
from evaluate.multiformat_ready_tree_fs import scan_tree
from evaluate.multiformat_ready_tree_types import TreeIdentityError
from evaluate.multiformat_schema import JsonValue


@dataclass(frozen=True, slots=True)
class TreeIdentity:
    """The canonical digest and physical byte counts for a corpus tree."""

    sha256: str
    entry_count: int
    total_bytes: int

    @property
    def files(self) -> int:
        """Return the number of regular files bound by this identity."""
        return self.entry_count

    @property
    def bytes(self) -> int:
        """Return the total size of regular files bound by this identity."""
        return self.total_bytes


def tree_identity(root: Path) -> TreeIdentity:
    """Hash every safe regular file beneath ``root`` using canonical framing."""
    records = scan_tree(root)
    ordered = sorted(records, key=lambda record: record.path.encode("utf-8"))
    files: list[JsonValue] = [
        {"path": record.path, "sha256": record.sha256, "size": record.size}
        for record in ordered
    ]
    payload: JsonValue = {
        "schema_version": 1,
        "files": files,
    }
    try:
        canonical = canonicalize(payload)
    except JcsError as error:
        raise TreeIdentityError(
            reason="tree identity canonicalization failed"
        ) from error
    return TreeIdentity(
        sha256=hashlib.sha256(canonical).hexdigest(),
        entry_count=len(ordered),
        total_bytes=sum(record.size for record in ordered),
    )


__all__: Final = [
    "TreeIdentity",
    "TreeIdentityError",
    "tree_identity",
]
