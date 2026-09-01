from __future__ import annotations

from dataclasses import dataclass
from enum import StrEnum
from pathlib import Path

from evaluate.multiformat_ready_types import ReadyInputPaths


class ReadyValidationError(ValueError):
    pass


class ReadyAssemblyFailure(StrEnum):
    INPUT_INVALID = "input-invalid"
    COPY_FAILED = "copy-failed"
    MANIFEST_INVALID = "manifest-invalid"
    VALIDATION_FAILED = "validation-failed"
    PUBLICATION_FAILED = "publication-failed"


@dataclass(frozen=True, slots=True)
class ReadyAssemblyError(Exception):
    failure: ReadyAssemblyFailure
    detail: str


@dataclass(frozen=True, slots=True)
class ReadyAssemblyInputs:
    sources: ReadyInputPaths
    output_dir: Path


@dataclass(frozen=True, slots=True)
class ReadyValidationInputs:
    sources: ReadyInputPaths
    corpus_root: Path


@dataclass(frozen=True, slots=True)
class ReadyAssemblySummary:
    status: str
    formats: int
    files: int
    sources: int
    supports: int
    tree_files: int
    tree_bytes: int
    tree_sha256: str
    manifest_sha256: str

    def to_json_value(self) -> dict[str, int | str]:
        return {
            "status": self.status,
            "formats": self.formats,
            "files": self.files,
            "sources": self.sources,
            "supports": self.supports,
            "tree_files": self.tree_files,
            "tree_bytes": self.tree_bytes,
            "tree_sha256": self.tree_sha256,
            "manifest_sha256": self.manifest_sha256,
        }
