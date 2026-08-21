from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path

from evaluate.multiformat_corpus_types import CorpusStatus, DocumentFormat


@dataclass(frozen=True, slots=True)
class AdmissionSource:
    document_format: DocumentFormat
    item_id: str
    track: str
    path: Path
    digest: str
    unit_count: int


AdmissionCheck = Callable[[AdmissionSource], bytes]


@dataclass(frozen=True, slots=True)
class AdmissionValidators:
    extraction: AdmissionCheck
    fonts: AdmissionCheck
    rendering: AdmissionCheck


@dataclass(frozen=True, slots=True)
class AdmissionMetadata:
    corpus_revision: str
    project_revision: str
    admitted_at: str


@dataclass(frozen=True, slots=True)
class AdmissionPlan:
    contract_path: Path
    corpus_manifests: tuple[Path, ...]
    destination: Path
    metadata: AdmissionMetadata


@dataclass(frozen=True, slots=True)
class AdmissionResult:
    status: CorpusStatus
    aggregate_sha256: str | None
    reasons: tuple[str, ...]
