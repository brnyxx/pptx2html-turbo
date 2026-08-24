from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass

from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_schema import JsonValue

TreeFetcher = Callable[[str, str], dict[str, JsonValue]]
BlobFetcher = Callable[[str, str, str], bytes]
EXCLUDED_PATH_TOKENS = (
    "bomb",
    "corrupt",
    "crash",
    "cve-",
    "encrypted",
    "exploit",
    "fuzz",
    "hang",
    "invalid",
    "malformed",
    "password",
    "protected",
)
MAX_SOURCE_BYTES = 64 * 1024 * 1024


class PublicPoolError(Exception):
    pass


@dataclass(frozen=True, slots=True)
class PublicSourceGroup:
    producer: str
    repository: str
    commit: str
    license_spdx: str
    quota: int
    path_prefixes: tuple[str, ...]
    static_paths: tuple[str, ...]


@dataclass(frozen=True, slots=True)
class PublicFormatPlan:
    document_format: DocumentFormat
    expected_count: int
    groups: tuple[PublicSourceGroup, ...]


@dataclass(frozen=True, slots=True)
class ValidatedPublicPoolSource:
    document_format: DocumentFormat
    source_id: str
    relative_path: str
    source_sha256: str
