from __future__ import annotations

import sys
from collections.abc import Callable
from enum import StrEnum
from pathlib import Path
from typing import NoReturn

from evaluate.multiformat_snapshot_publish import (
    SnapshotPublishError,
    publish_snapshot,
)

SnapshotWriter = Callable[[Path], None]


class SecurityPublishFailure(StrEnum):
    DESTINATION_EXISTS = "destination-exists"
    LOCKED = "locked"
    PUBLICATION_FAILED = "publication-failed"


class SecurityPublishError(Exception):
    __slots__ = ("failure", "path")

    def __init__(self, path: Path, failure: SecurityPublishFailure) -> None:
        self.path = path
        self.failure = failure
        super().__init__(path, failure)

    def __str__(self) -> str:
        return f"security snapshot publication failed: {self.failure.value}"


def publish_security_snapshot(
    destination: Path,
    writer: SnapshotWriter,
) -> None:
    """Publish a security snapshot through the format-neutral publisher."""

    try:
        try:
            publish_snapshot(
                destination,
                writer,
                lock_namespace="security-sources",
            )
        finally:
            active_error = sys.exception()
            if active_error is not None and hasattr(active_error, "__notes__"):
                for index, note in enumerate(active_error.__notes__):
                    prefix = "snapshot cleanup failed:"
                    if note.startswith(prefix):
                        active_error.__notes__[index] = (
                            f"security snapshot cleanup failed:{note[len(prefix) :]}"
                        )
    except SnapshotPublishError as error:
        _raise_security_error(error)


def _raise_security_error(error: SnapshotPublishError) -> NoReturn:
    security_error = SecurityPublishError(
        error.path,
        SecurityPublishFailure(error.failure.value),
    )
    if hasattr(error, "__notes__"):
        for note in error.__notes__:
            security_error.add_note(note)
    if error.__cause__ is not None:
        raise security_error from error.__cause__
    raise security_error from None
