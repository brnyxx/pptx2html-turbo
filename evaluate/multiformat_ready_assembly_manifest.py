from __future__ import annotations

from pathlib import Path
from typing import cast

from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_ready_tree import TreeIdentity
from evaluate.multiformat_ready_types import ReadyInputPaths, ReadySupport
from evaluate.multiformat_schema import JsonValue, sha256_file

UPSTREAM_PATHS = (
    ("docx-conformance", "docx_conformance"),
    ("legacy-binary-config", "legacy_binary_config"),
    ("legacy-binary-manifest", "legacy_binary_manifest"),
    ("legacy-conformance", "legacy_conformance"),
    ("pdf-conformance", "pdf_conformance"),
    ("pptx-conformance", "pptx_conformance"),
    ("public-config", "public_config"),
    ("public-pool-manifest", "public_pool_manifest"),
    ("security-manifest", "security_manifest"),
    ("xlsx-conformance", "xlsx_conformance"),
)


def upstream_bindings(paths: ReadyInputPaths) -> list[JsonValue]:
    return [
        {"role": role, "sha256": sha256_file(cast(Path, getattr(paths, field)))}
        for role, field in UPSTREAM_PATHS
    ]


def corpus_bindings(
    root: Path, supports: tuple[ReadySupport, ...]
) -> dict[str, JsonValue]:
    support_counts = {
        document_format: sum(item.owner_format is document_format for item in supports)
        for document_format in DocumentFormat
    }
    result: dict[str, JsonValue] = {}
    for document_format in DocumentFormat:
        relative = f"corpora/{document_format.value}/manifest.json"
        result[document_format.value] = {
            "path": relative,
            "sha256": sha256_file(root / relative),
            "conformance_units": 100,
            "blind_files": 75,
            "security_cases": 10,
            "support_files": support_counts[document_format],
        }
    return result


def support_relations(supports: tuple[ReadySupport, ...]) -> list[JsonValue]:
    return [
        {
            "owner_format": item.owner_format.value,
            "owner_source_id": item.owner_source_id,
            "support_format": item.support_format.value,
            "modern_case_id": item.modern_case_id,
            "support_id": item.support_id,
            "path": (
                f"corpora/{item.owner_format.value}/sources/support/{item.filename}"
            ),
            "sha256": item.source_sha256,
        }
        for item in sorted(
            supports,
            key=lambda value: (
                value.owner_format.value,
                value.owner_source_id,
                value.support_id,
            ),
        )
    ]


def build_assembly_manifest(
    paths: ReadyInputPaths,
    root: Path,
    supports: tuple[ReadySupport, ...],
    tree: TreeIdentity,
) -> dict[str, JsonValue]:
    return {
        "schema_version": 1,
        "status": "VALIDATED",
        "contract_sha256": sha256_file(paths.contract),
        "plan": {
            "path": "conformance-plan.json",
            "sha256": sha256_file(root / "conformance-plan.json"),
        },
        "native_inventory": {
            "path": "native-unit-inventory.json",
            "sha256": sha256_file(root / "native-unit-inventory.json"),
        },
        "upstream_manifests": upstream_bindings(paths),
        "corpora": corpus_bindings(root, supports),
        "support_relations": support_relations(supports),
        "tree": {
            "files": tree.files,
            "bytes": tree.bytes,
            "sha256": tree.sha256,
        },
    }
