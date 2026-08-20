from __future__ import annotations

from dataclasses import dataclass
from enum import StrEnum
from typing import TypedDict


class DocumentFormat(StrEnum):
    PPTX = "pptx"
    DOCX = "docx"
    DOC = "doc"
    XLSX = "xlsx"
    XLS = "xls"
    PPT = "ppt"
    PDF = "pdf"


class CorpusStatus(StrEnum):
    READY = "READY"
    INCOMPLETE = "INCOMPLETE"


class SecurityOutcome(StrEnum):
    REJECT = "reject"
    SAFE_CONVERT = "safe-convert"


class CorpusError(Exception):
    __slots__ = ("reason", "detail")

    def __init__(self, reason: str, detail: str) -> None:
        self.reason = reason
        self.detail = detail
        super().__init__(reason, detail)

    def __str__(self) -> str:
        return f"{self.reason}: {self.detail}"


@dataclass(frozen=True, slots=True)
class SourceRecord:
    item_id: str
    relative_path: str
    digest: str


@dataclass(frozen=True, slots=True)
class TrackValidation:
    count: int
    item_ids: frozenset[str]
    source_paths: frozenset[str]
    source_hashes: frozenset[str]


@dataclass(frozen=True, slots=True)
class CorpusRules:
    conformance_units: int
    blind_files: int
    security_cases: int
    quotas: dict[str, int]
    security_outcomes: dict[str, SecurityOutcome]
    paired_quotas: dict[str, int] | None
    paired_format: DocumentFormat | None


class CorpusValidationJson(TypedDict):
    schema_version: int
    status: str
    format: str
    conformance_units: int
    blind_files: int
    security_cases: int
    blind_producers: int


@dataclass(frozen=True, slots=True)
class CorpusValidation:
    status: CorpusStatus
    document_format: DocumentFormat
    conformance_units: int
    blind_files: int
    security_cases: int
    blind_producers: int

    def to_json_value(self) -> CorpusValidationJson:
        return {
            "schema_version": 1,
            "status": self.status.value,
            "format": self.document_format.value,
            "conformance_units": self.conformance_units,
            "blind_files": self.blind_files,
            "security_cases": self.security_cases,
            "blind_producers": self.blind_producers,
        }
