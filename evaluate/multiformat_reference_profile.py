from __future__ import annotations

from dataclasses import dataclass
from enum import StrEnum
from pathlib import Path

from evaluate.multiformat_evidence import oracle_lock_ready
from evaluate.multiformat_schema import (
    integer_value,
    sha256_file,
    string_value,
)
from evaluate.multiformat_strict_json import read_strict_object


class ReferenceProfile(StrEnum):
    LIBREOFFICE_POPPLER = "libreoffice-poppler"
    MICROSOFT_OFFICE = "microsoft-office"


@dataclass(frozen=True, slots=True)
class ReferenceLockIdentity:
    schema_version: int
    profile: ReferenceProfile
    sha256: str


class ReferenceProfileError(ValueError):
    """Raised when a reference lock has an unsupported identity."""


def load_reference_lock_identity(path: Path) -> ReferenceLockIdentity:
    """Load a reference lock identity without modifying the lock bytes."""
    try:
        values = read_strict_object(path)
        schema_version = integer_value(values, "schema_version")
        if schema_version == 1:
            if "reference_profile" in values or not oracle_lock_ready(path):
                raise ReferenceProfileError("legacy Office lock is not valid")
            profile = ReferenceProfile.MICROSOFT_OFFICE
        elif schema_version == 2:
            try:
                profile = ReferenceProfile(
                    string_value(values, "reference_profile"),
                )
            except (TypeError, ValueError) as error:
                raise ReferenceProfileError(
                    "reference lock profile is unsupported or missing",
                ) from error
        else:
            raise ReferenceProfileError("reference lock schema is unsupported")
        return ReferenceLockIdentity(
            schema_version=schema_version,
            profile=profile,
            sha256=sha256_file(path),
        )
    except ReferenceProfileError:
        raise
    except (OSError, UnicodeError, TypeError, ValueError) as error:
        raise ReferenceProfileError("reference lock identity is invalid") from error
