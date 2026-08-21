from __future__ import annotations

import argparse
import http.client
import json
import os
import urllib.error
import urllib.request
from pathlib import Path

from evaluate.multiformat_public_pool import (
    PublicPoolError,
    collect_public_pool,
)
from evaluate.multiformat_public_pool_sources import public_source_url
from evaluate.multiformat_public_pool_types import MAX_SOURCE_BYTES
from evaluate.multiformat_schema import JsonValue


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Collect a pinned multi-repository blind source pool.",
    )
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    arguments = parser.parse_args()
    try:
        collect_public_pool(
            arguments.config,
            arguments.output_dir,
            tree_fetcher=_fetch_tree,
            blob_fetcher=_fetch_blob,
        )
    except PublicPoolError as error:
        parser.error(str(error))


def _fetch_tree(repository: str, commit: str) -> dict[str, JsonValue]:
    value = _fetch(
        f"https://api.github.com/repos/{repository}/git/trees/{commit}?recursive=1",
        MAX_SOURCE_BYTES,
    )
    try:
        parsed = json.loads(value)
    except json.JSONDecodeError as error:
        raise PublicPoolError("GitHub tree response is invalid") from error
    if not isinstance(parsed, dict):
        raise PublicPoolError("GitHub tree response is not an object")
    return parsed


def _fetch_blob(repository: str, commit: str, path: str) -> bytes:
    return _fetch(_raw_url(repository, commit, path), MAX_SOURCE_BYTES)


def _raw_url(repository: str, commit: str, path: str) -> str:
    return public_source_url(repository, commit, path)


def _fetch(url: str, limit: int) -> bytes:
    headers = {
        "Accept": "application/vnd.github+json",
        "User-Agent": "document2html-corpus-collector",
        "X-GitHub-Api-Version": "2022-11-28",
    }
    token = os.environ.get("GITHUB_TOKEN")
    if token:
        headers["Authorization"] = f"Bearer {token}"
    request = urllib.request.Request(url, headers=headers)
    try:
        with urllib.request.urlopen(request, timeout=60) as response:
            value = response.read(limit + 1)
    except (
        http.client.HTTPException,
        OSError,
        urllib.error.URLError,
    ) as error:
        raise PublicPoolError("public source download failed") from error
    if len(value) > limit:
        raise PublicPoolError("public source download exceeds the bound")
    return value


if __name__ == "__main__":
    main()
