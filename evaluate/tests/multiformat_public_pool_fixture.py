from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path

from evaluate.multiformat_public_pool import collect_public_pool
from evaluate.multiformat_schema import JsonValue
from evaluate.tests.multiformat_source_fixture import write_positive_source


@dataclass(frozen=True, slots=True)
class PublicPoolFixture:
    config: Path
    manifest: Path


def write_public_pool_fixture(root: Path) -> PublicPoolFixture:
    config = _write_config(root)
    blobs = _write_blobs(root)
    manifest = collect_public_pool(
        config,
        root / "pool",
        tree_fetcher=_tree,
        blob_fetcher=lambda repository, commit, path: blobs[(repository, path)],
    )
    return PublicPoolFixture(config, manifest)


def _write_config(root: Path) -> Path:
    config = root / "config.json"
    config.write_text(
        json.dumps(
            {
                "schema_version": 1,
                "formats": {
                    "docx": {
                        "expected_count": 5,
                        "groups": [
                            {
                                "producer": f"producer-{index}",
                                "repository": f"owner/repo-{index}",
                                "commit": str(index + 1) * 40,
                                "license_spdx": "MIT",
                                "quota": 1,
                                "path_prefixes": ["fixtures/"],
                                "static_paths": [],
                            }
                            for index in range(5)
                        ],
                    }
                },
            },
            sort_keys=True,
        ),
        encoding="utf-8",
    )
    return config


def _write_blobs(root: Path) -> dict[tuple[str, str], bytes]:
    result: dict[tuple[str, str], bytes] = {}
    for index in range(5):
        path = root / f"valid-{index}.docx"
        write_positive_source(path, "docx", f"public-{index}")
        valid_name = "fixtures/valid file.docx" if index == 4 else "fixtures/valid.docx"
        result[(f"owner/repo-{index}", valid_name)] = path.read_bytes()
    return result


def _tree(repository: str, commit: str) -> dict[str, JsonValue]:
    return {
        "truncated": False,
        "tree": [
            {
                "path": (
                    "fixtures/valid file.docx"
                    if repository == "owner/repo-4"
                    else "fixtures/valid.docx"
                ),
                "type": "blob",
                "size": 1024,
            }
        ],
    }
