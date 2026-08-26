from __future__ import annotations

from dataclasses import dataclass
from enum import StrEnum
from pathlib import Path

from evaluate.multiformat_evidence import oracle_lock_ready
from evaluate.multiformat_portable_lock import (
    PortableLockError,
    PortableLockIncompleteError,
    PortableReferenceLockIdentity,
    validate_reference_lock,
)
from evaluate.multiformat_reference_profile import (
    ReferenceProfile,
    ReferenceProfileError,
    load_reference_lock_identity,
)
from evaluate.multiformat_schema import JsonValue


class GateStatus(StrEnum):
    PASS = "PASS"
    FAIL = "FAIL"
    INCOMPLETE = "INCOMPLETE"


@dataclass(frozen=True, slots=True)
class FormatOracleLock:
    format: str
    path: Path


@dataclass(frozen=True, slots=True)
class ResolvedOracleLock:
    path: Path
    sha256: str
    portable: PortableReferenceLockIdentity | None


class OracleLockInputError(ValueError):
    def __init__(self, status: GateStatus, reason: str) -> None:
        super().__init__(reason)
        self.status = status
        self.reason = reason


@dataclass(frozen=True, slots=True)
class OracleLockInput:
    shared: Path | None = None
    format_locks: tuple[FormatOracleLock, ...] = ()
    directory: Path | None = None

    @classmethod
    def per_format(cls, values: dict[str, Path]) -> OracleLockInput:
        return cls(
            format_locks=tuple(FormatOracleLock(*item) for item in values.items())
        )

    @classmethod
    def lock_directory(cls, path: Path) -> OracleLockInput:
        return cls(directory=path)

    def resolve(
        self,
        required_formats: list[str],
        evidence_root: Path,
    ) -> dict[str, ResolvedOracleLock]:
        mode_count = sum(
            (
                self.shared is not None,
                bool(self.format_locks),
                self.directory is not None,
            )
        )
        if mode_count != 1:
            raise OracleLockInputError(GateStatus.FAIL, "oracle_locks")
        if self.shared is not None:
            return self._resolve_shared(required_formats)
        return self._resolve_per_format(required_formats, evidence_root)

    def _resolve_shared(self, formats: list[str]) -> dict[str, ResolvedOracleLock]:
        if self.shared is None:
            # `resolve` selects this branch only when a shared lock is set, so
            # a missing path is a programming error rather than bad input.
            raise OracleLockInputError(GateStatus.FAIL, "oracle_locks")
        try:
            identity = load_reference_lock_identity(self.shared)
        except ReferenceProfileError as error:
            raise OracleLockInputError(GateStatus.INCOMPLETE, "oracle_lock") from error
        if (
            identity.schema_version != 1
            or identity.profile is not ReferenceProfile.MICROSOFT_OFFICE
            or not oracle_lock_ready(self.shared)
        ):
            raise OracleLockInputError(GateStatus.FAIL, "oracle_lock")
        lock = ResolvedOracleLock(self.shared, identity.sha256, None)
        return dict.fromkeys(formats, lock)

    def _resolve_per_format(
        self,
        formats: list[str],
        evidence_root: Path,
    ) -> dict[str, ResolvedOracleLock]:
        if self.directory is not None:
            if not self.directory.is_dir() or self.directory.is_symlink():
                raise OracleLockInputError(GateStatus.INCOMPLETE, "oracle_lock_missing")
            actual = {path.stem for path in self.directory.glob("*.json")}
            if actual - set(formats):
                raise OracleLockInputError(GateStatus.FAIL, "oracle_lock_extra")
            entries = tuple(
                FormatOracleLock(name, self.directory / f"{name}.json")
                for name in formats
            )
        else:
            entries = self.format_locks
        names = [entry.format for entry in entries]
        if len(names) != len(set(names)):
            raise OracleLockInputError(GateStatus.FAIL, "oracle_lock_duplicate")
        missing = set(formats) - set(names)
        if missing:
            raise OracleLockInputError(GateStatus.INCOMPLETE, "oracle_lock_missing")
        if set(names) - set(formats):
            raise OracleLockInputError(GateStatus.FAIL, "oracle_lock_extra")
        result: dict[str, ResolvedOracleLock] = {}
        for entry in entries:
            if not entry.path.is_file() or entry.path.is_symlink():
                raise OracleLockInputError(GateStatus.INCOMPLETE, "oracle_lock_missing")
            try:
                identity = validate_reference_lock(entry.path, evidence_root)
            except PortableLockIncompleteError as error:
                raise OracleLockInputError(
                    GateStatus.INCOMPLETE, "oracle_lock"
                ) from error
            except PortableLockError as error:
                raise OracleLockInputError(GateStatus.FAIL, "oracle_lock") from error
            if identity.scope_format != entry.format:
                raise OracleLockInputError(GateStatus.FAIL, "oracle_lock_scope")
            result[entry.format] = ResolvedOracleLock(
                entry.path, identity.sha256, identity
            )
        paths = {lock.path.resolve() for lock in result.values()}
        hashes = {lock.sha256 for lock in result.values()}
        if len(paths) != len(formats) or len(hashes) != len(formats):
            raise OracleLockInputError(GateStatus.FAIL, "oracle_lock_shared")
        return result


@dataclass(frozen=True, slots=True)
class FormatGateResult:
    format: str
    status: GateStatus
    reasons: tuple[str, ...]


@dataclass(frozen=True, slots=True)
class GateSummary:
    status: GateStatus
    formats: tuple[FormatGateResult, ...]

    def to_json_value(self) -> dict[str, JsonValue]:
        return {
            "status": self.status.value,
            "formats": [
                {
                    "format": result.format,
                    "status": result.status.value,
                    "reasons": list(result.reasons),
                }
                for result in self.formats
            ],
        }
