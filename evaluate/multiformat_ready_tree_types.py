"""Typed records shared by READY tree identity filesystem operations."""

from __future__ import annotations

import os
from dataclasses import dataclass


class TreeIdentityError(ValueError):
    reason: str

    def __init__(self, reason: str) -> None:
        super().__init__(reason)
        self.reason = reason


@dataclass(frozen=True, slots=True)
class TreeFileRecord:
    path: str
    sha256: str
    size: int


@dataclass(frozen=True, slots=True)
class DirectoryContext:
    fd: int
    relative_parts: tuple[str, ...]
    expected: os.stat_result
    parent_fd: int | None
