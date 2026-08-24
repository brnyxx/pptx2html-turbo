from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Protocol

from evaluate.multiformat_corpus_types import DocumentFormat

LEGACY_PAIRS = (
    (DocumentFormat.DOC, DocumentFormat.DOCX),
    (DocumentFormat.XLS, DocumentFormat.XLSX),
    (DocumentFormat.PPT, DocumentFormat.PPTX),
)


class LegacyConformanceError(Exception):
    pass


@dataclass(frozen=True, slots=True)
class LegacyPairGeneration:
    contract: Path
    plan: Path
    modern_manifests: tuple[Path, ...]
    output_dir: Path


@dataclass(frozen=True, slots=True)
class LegacyPairJob:
    case_id: str
    document_format: DocumentFormat
    source: Path
    destination: Path
    workspace: Path


class LegacyPairMaterializer(Protocol):
    def __call__(self, job: LegacyPairJob) -> int: ...


@dataclass(frozen=True, slots=True)
class LegacyToolIdentity:
    soffice_sha256: str
    soffice_version: str
    pdfinfo_sha256: str
    pdfinfo_version: str
    font_environment_sha256: str


@dataclass(frozen=True, slots=True)
class LegacyPairRuntime:
    materialize: LegacyPairMaterializer
    tools: LegacyToolIdentity


@dataclass(frozen=True, slots=True)
class ModernSource:
    item_id: str
    path: Path
    digest: str
    primary_stratum: str


@dataclass(frozen=True, slots=True)
class ModernSnapshot:
    document_format: DocumentFormat
    manifest_sha256: str
    sources: tuple[ModernSource, ...]
