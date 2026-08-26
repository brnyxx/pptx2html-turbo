from __future__ import annotations

import os
from pathlib import Path

from evaluate.jcs import canonicalize
from evaluate.multiformat_ready_assembly_types import ReadyValidationError
from evaluate.multiformat_schema import JsonValue
from evaluate.multiformat_strict_json import read_strict_object

ROOT_FIELDS = {
    "schema_version",
    "status",
    "contract_sha256",
    "plan",
    "native_inventory",
    "upstream_manifests",
    "corpora",
    "support_relations",
    "tree",
}


def read_canonical_object(path: Path) -> dict[str, JsonValue]:
    value = read_strict_object(path)
    if path.read_bytes() != canonicalize(value) + b"\n":
        raise ReadyValidationError(f"non-canonical JSON: {path.name}")
    return value


def read_root_manifest(root: Path) -> dict[str, JsonValue]:
    path = root / "assembly-manifest.json"
    value = read_canonical_object(path)
    if set(value) != ROOT_FIELDS:
        raise ReadyValidationError("assembly manifest fields")
    if value.get("schema_version") != 1 or value.get("status") != "VALIDATED":
        raise ReadyValidationError("assembly manifest status")
    if os.path.lexists(root / "READY"):
        raise ReadyValidationError("aggregate READY marker is forbidden")
    _binding(value.get("plan"), "conformance-plan.json")
    _binding(value.get("native_inventory"), "native-unit-inventory.json")
    upstream = value.get("upstream_manifests")
    if not isinstance(upstream, list) or len(upstream) != 10:
        raise ReadyValidationError("upstream manifest bindings")
    roles: list[str] = []
    for item in upstream:
        if not isinstance(item, dict) or set(item) != {"role", "sha256"}:
            raise ReadyValidationError("upstream manifest binding fields")
        role = item.get("role")
        _digest(item.get("sha256"))
        if not isinstance(role, str):
            raise ReadyValidationError("upstream manifest role")
        roles.append(role)
    if roles != sorted(roles) or len(set(roles)) != 10:
        raise ReadyValidationError("upstream manifest role order")
    corpora = value.get("corpora")
    if not isinstance(corpora, dict) or len(corpora) != 7:
        raise ReadyValidationError("corpus bindings")
    relations = value.get("support_relations")
    if not isinstance(relations, list) or len(relations) != 180:
        raise ReadyValidationError("support relations")
    tree = value.get("tree")
    if not isinstance(tree, dict) or set(tree) != {"files", "bytes", "sha256"}:
        raise ReadyValidationError("tree binding")
    if tree.get("files") != 1484 or not isinstance(tree.get("bytes"), int):
        raise ReadyValidationError("tree counts")
    _digest(tree.get("sha256"))
    return value


def _binding(value: JsonValue | None, path: str) -> None:
    if not isinstance(value, dict) or set(value) != {"path", "sha256"}:
        raise ReadyValidationError("artifact binding fields")
    if value.get("path") != path:
        raise ReadyValidationError("artifact binding path")
    _digest(value.get("sha256"))


def _digest(value: JsonValue | None) -> None:
    if not isinstance(value, str) or len(value) != 64:
        raise ReadyValidationError("invalid SHA-256")
    if any(character not in "0123456789abcdef" for character in value):
        raise ReadyValidationError("invalid SHA-256")
