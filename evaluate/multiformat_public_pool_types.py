from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass

from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_schema import JsonValue

TreeFetcher = Callable[[str, str], dict[str, JsonValue]]
BlobFetcher = Callable[[str, str, str], bytes]
EXCLUDED_PATH_TOKENS = (
    "(enc)",
    "bomb",
    "corrupt",
    "crash",
    "cve-",
    "ddelink",
    "encrypted",
    "exploit",
    "fuzz",
    "hang",
    "invalid",
    "malformed",
    "password",
    "protected",
)
EXCLUDED_REPOSITORY_PATHS = frozenset(
    {
        "Examples/Data/Presentations/Properties/open_pass1.ppt",
        "spec/integration/data/huge.xlsx",
        "test-data/slideshow/2100a8d44da546f97ab7795c500a58bed6cb655d.ppt",
        "test-data/slideshow/60f557c0a46bcb0068b1c3e15589dac383307bc8.ppt",
        "testcases/test-data/slideshow/backgrounds.ppt",
        (
            "tika-parsers/tika-parsers-standard/"
            "tika-parsers-standard-modules/tika-parser-microsoft-module/"
            "src/test/resources/test-documents/pictures.ppt"
        ),
    }
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
