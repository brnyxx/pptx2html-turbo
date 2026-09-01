from __future__ import annotations

from pathlib import Path

from evaluate.multiformat_ready_types import (
    ReadyInputPaths,
    ReadySource,
    ReadySupport,
)
from evaluate.multiformat_schema import sha256_file

FileIdentity = tuple[int, int, int, str]


def file_identity(path: Path) -> FileIdentity:
    value = path.lstat()
    return value.st_dev, value.st_ino, value.st_size, sha256_file(path)


def source_file_identities(
    sources: list[ReadySource],
    supports: list[ReadySupport],
) -> dict[Path, FileIdentity]:
    paths = {
        *(item.source_path for item in sources),
        *(item.source_path for item in supports),
    }
    return {path: file_identity(path) for path in paths}


def identities_unchanged(values: dict[Path, FileIdentity]) -> bool:
    return all(file_identity(path) == expected for path, expected in values.items())


def input_roots(paths: ReadyInputPaths) -> dict[str, Path]:
    return {
        "pptx-conformance": paths.pptx_conformance.parent,
        "docx-conformance": paths.docx_conformance.parent,
        "xlsx-conformance": paths.xlsx_conformance.parent,
        "pdf-conformance": paths.pdf_conformance.parent,
        "legacy-conformance": paths.legacy_conformance.parent,
        "legacy-binary": paths.legacy_binary_manifest.parent,
        "public-pool": paths.public_pool_manifest.parent,
        "security": paths.security_manifest.parent,
        "native-inventory": paths.native_inventory_root,
    }
