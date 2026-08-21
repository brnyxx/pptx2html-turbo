from __future__ import annotations

import hashlib
import urllib.parse
from pathlib import Path

from evaluate.multiformat_corpus_sources import validate_source
from evaluate.multiformat_corpus_types import CorpusError, DocumentFormat
from evaluate.multiformat_public_pool_config import validate_repository_path
from evaluate.multiformat_public_pool_types import (
    EXCLUDED_PATH_TOKENS,
    MAX_SOURCE_BYTES,
    BlobFetcher,
    PublicPoolError,
    PublicSourceGroup,
)
from evaluate.multiformat_schema import JsonValue, sha256_file


def public_source_url(repository: str, commit: str, path: str) -> str:
    encoded_path = urllib.parse.quote(path, safe="/")
    return f"https://raw.githubusercontent.com/{repository}/{commit}/{encoded_path}"


def collect_public_source_group(
    root: Path,
    document_format: DocumentFormat,
    group: PublicSourceGroup,
    tree: dict[str, JsonValue],
    blob_fetcher: BlobFetcher,
    seen_hashes: set[str],
) -> list[dict[str, JsonValue]]:
    result: list[dict[str, JsonValue]] = []
    for repository_path in _candidate_paths(tree, document_format, group):
        value = blob_fetcher(group.repository, group.commit, repository_path)
        if not value or len(value) > MAX_SOURCE_BYTES:
            continue
        destination = (
            root
            / "sources"
            / document_format.value
            / group.producer
            / f"{len(result) + 1:03d}.{document_format.value}"
        )
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_bytes(value)
        digest = sha256_file(destination)
        if digest in seen_hashes or not _valid_source(
            destination,
            root,
            document_format,
            digest,
        ):
            destination.unlink()
            continue
        seen_hashes.add(digest)
        result.append(
            _source_value(
                root,
                destination,
                document_format,
                group,
                repository_path,
                digest,
            )
        )
        if len(result) == group.quota:
            return result
    raise PublicPoolError(f"public pool quota unavailable: {group.producer}")


def _candidate_paths(
    tree: dict[str, JsonValue],
    document_format: DocumentFormat,
    group: PublicSourceGroup,
) -> tuple[str, ...]:
    truncated = tree.get("truncated")
    if not isinstance(truncated, bool):
        raise PublicPoolError("public pool tree metadata is invalid")
    if truncated and not group.static_paths:
        raise PublicPoolError("public pool tree is truncated")
    entries = tree.get("tree")
    if not isinstance(entries, list) or any(
        not isinstance(item, dict) for item in entries
    ):
        raise PublicPoolError("public pool tree is invalid")
    available: dict[str, int] = {}
    for item in entries:
        path = item.get("path")
        item_type = item.get("type")
        size = item.get("size")
        if isinstance(path, str) and item_type == "blob" and isinstance(size, int):
            available[path] = size
    candidates = group.static_paths or tuple(sorted(available))
    result = []
    suffix = f".{document_format.value}"
    for path in candidates:
        lower = path.lower()
        if (
            path not in available
            or not 0 < available[path] <= MAX_SOURCE_BYTES
            or not lower.endswith(suffix)
            or any(token in lower for token in EXCLUDED_PATH_TOKENS)
            or (
                group.path_prefixes
                and not any(path.startswith(prefix) for prefix in group.path_prefixes)
            )
        ):
            continue
        validate_repository_path(path)
        result.append(path)
    return tuple(result)


def _valid_source(
    path: Path,
    root: Path,
    document_format: DocumentFormat,
    digest: str,
) -> bool:
    try:
        validate_source(
            {
                "id": "public-source",
                "path": path.relative_to(root).as_posix(),
                "sha256": digest,
            },
            root,
            document_format,
            require_valid_format=True,
        )
    except CorpusError:
        return False
    return True


def _source_value(
    root: Path,
    path: Path,
    document_format: DocumentFormat,
    group: PublicSourceGroup,
    repository_path: str,
    digest: str,
) -> dict[str, JsonValue]:
    template = hashlib.sha256(
        f"{group.repository}:{repository_path}".encode()
    ).hexdigest()[:20]
    index = path.stem
    return {
        "id": f"blind-{document_format.value}-{group.producer}-{index}",
        "path": path.relative_to(root).as_posix(),
        "sha256": digest,
        "producer": group.producer,
        "source_uri": public_source_url(
            group.repository,
            group.commit,
            repository_path,
        ),
        "template_family": f"{group.producer}-{template}",
        "repository": group.repository,
        "commit": group.commit,
        "repository_path": repository_path,
        "license_spdx": group.license_spdx,
        "applicable_metrics": ["visual", "content", "layout"],
        "background": "light",
    }
