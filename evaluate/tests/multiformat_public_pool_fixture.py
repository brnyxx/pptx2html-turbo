from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path

from evaluate.multiformat_candidate_artifacts import write_canonical_json
from evaluate.multiformat_public_pool import collect_public_pool
from evaluate.multiformat_schema import JsonValue, object_value, sha256_file
from evaluate.multiformat_source_fixture import write_positive_source
from evaluate.multiformat_strict_json import read_strict_object


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


def write_public_pool_config(root: Path) -> Path:
    return _write_config(root)


def write_public_pool_blobs(root: Path) -> dict[tuple[str, str], bytes]:
    return _write_blobs(root)


def public_pool_tree(repository: str, commit: str) -> dict[str, JsonValue]:
    return _tree(repository, commit)


def write_multiformat_public_pool_fixture(root: Path) -> PublicPoolFixture:
    fixture = write_public_pool_fixture(root)
    config_values = read_strict_object(fixture.config)
    formats = object_value(config_values, "formats")
    formats["pdf"] = {
        "expected_count": 5,
        "groups": [_pdf_group(index) for index in range(5)],
    }
    _ = fixture.config.write_text(
        json.dumps(config_values, sort_keys=True),
        encoding="utf-8",
    )
    values = read_strict_object(fixture.manifest)
    formats = object_value(values, "formats")
    pdf_sources: list[JsonValue] = []
    for index in range(5):
        producer = f"pdf-producer-{index}"
        repository = f"owner/pdf-repo-{index}"
        commit = f"{index + 6:x}" * 40
        relative_path = f"sources/pdf/{producer}/001.pdf"
        path = fixture.manifest.parent / relative_path
        _ = path.parent.mkdir(parents=True, exist_ok=True)
        write_positive_source(path, "pdf", f"pdf-{index}")
        pdf_sources.append(
            {
                "id": f"blind-pdf-{producer}-001",
                "path": relative_path,
                "sha256": sha256_file(path),
                "producer": producer,
                "source_uri": f"https://example.com/{index}",
                "template_family": f"{producer}-template",
                "repository": repository,
                "commit": commit,
                "repository_path": "fixtures/valid.pdf",
                "license_spdx": "MIT",
                "applicable_metrics": ["visual", "content", "layout"],
                "background": "light",
            }
        )
    formats["pdf"] = {"expected_count": 5, "sources": pdf_sources}
    write_canonical_json(fixture.manifest, values)
    return fixture


def _pdf_group(index: int) -> dict[str, JsonValue]:
    return {
        "producer": f"pdf-producer-{index}",
        "repository": f"owner/pdf-repo-{index}",
        "commit": f"{index + 6:x}" * 40,
        "license_spdx": "MIT",
        "quota": 1,
        "path_prefixes": ["fixtures/"],
        "static_paths": [],
    }


def _write_config(root: Path) -> Path:
    config = root / "config.json"
    _ = config.write_text(
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


def _tree(repository: str, _commit: str) -> dict[str, JsonValue]:
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
