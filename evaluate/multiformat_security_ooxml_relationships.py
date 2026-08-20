from __future__ import annotations

import posixpath
from pathlib import Path
from typing import Final
from xml.etree import ElementTree

from evaluate.multiformat_package_validation import RELATIONSHIPS_NS
from evaluate.multiformat_security_cfb import has_cfb_macro_storage, is_coherent_cfb

RELATIONSHIP_TAG: Final[str] = f"{{{RELATIONSHIPS_NS}}}Relationship"
MACRO_RELATIONSHIP: Final[str] = "/vbaProject"
EMBEDDED_RELATIONSHIPS: Final[frozenset[str]] = frozenset({"/oleObject", "/package"})


def parse_relationships(
    values: dict[str, bytes],
) -> list[tuple[str, str, str, bool]]:
    result: list[tuple[str, str, str, bool]] = []
    for name, value in values.items():
        if not name.lower().endswith(".rels") or _contains_entity(value):
            continue
        try:
            root = ElementTree.fromstring(value)
        except ElementTree.ParseError:
            continue
        source = _relationship_source(name)
        for child in root:
            if child.tag != RELATIONSHIP_TAG:
                continue
            target = child.attrib.get("Target")
            relationship_type = child.attrib.get("Type")
            if target is None or relationship_type is None:
                continue
            external = child.attrib.get("TargetMode", "Internal") == "External"
            result.append((source, target, relationship_type, external))
    return result


def reachable_office_parts(
    names: set[str],
    relationships: list[tuple[str, str, str, bool]],
) -> set[str]:
    main_parts: list[str] = []
    graph: dict[str, set[str]] = {}
    for source, target, relationship_type, external in relationships:
        if external:
            continue
        destination = target_part(source, target)
        if destination not in names:
            continue
        graph.setdefault(source, set()).add(destination)
        if source == "" and relationship_type.endswith("/officeDocument"):
            main_parts.append(destination)
    pending = [part for part in main_parts if part in names]
    reachable = set(pending)
    while pending:
        source = pending.pop()
        for destination in graph.get(source, set()):
            if destination not in reachable:
                reachable.add(destination)
                pending.append(destination)
    return reachable


def has_macro(
    values: dict[str, bytes],
    relationships: list[tuple[str, str, str, bool]],
    relationship_sources: set[str],
) -> bool:
    lower_values = {name.lower(): value for name, value in values.items()}
    for source, target, relationship_type, external in relationships:
        part = target_part(source, target).lower()
        if (
            not external
            and source in relationship_sources
            and relationship_type.endswith(MACRO_RELATIONSHIP)
            and part.endswith("/vbaproject.bin")
            and part in lower_values
            and has_cfb_macro_storage(lower_values[part])
        ):
            return True
    return False


def has_embedded_object(
    values: dict[str, bytes],
    relationships: list[tuple[str, str, str, bool]],
    relationship_sources: set[str],
) -> bool:
    lower_values = {name.lower(): value for name, value in values.items()}
    for source, target, relationship_type, external in relationships:
        part = target_part(source, target).lower()
        if (
            not external
            and source in relationship_sources
            and any(relationship_type.endswith(kind) for kind in EMBEDDED_RELATIONSHIPS)
            and "/embeddings/" in f"/{part}"
            and part in lower_values
            and is_coherent_cfb(lower_values[part])
        ):
            return True
    return False


def has_relationship_cycle(
    names: set[str],
    relationships: list[tuple[str, str, str, bool]],
    relationship_sources: set[str],
) -> bool:
    graph: dict[str, set[str]] = {}
    for source, target, _, external in relationships:
        destination = target_part(source, target)
        if (
            not external
            and source in relationship_sources
            and source
            and destination in names
        ):
            graph.setdefault(source, set()).add(destination)
    nodes = set(graph)
    nodes.update(child for children in graph.values() for child in children)
    indegree = {node: 0 for node in nodes}
    for children in graph.values():
        for child in children:
            indegree[child] += 1
    pending = [node for node, degree in indegree.items() if degree == 0]
    removed = 0
    while pending:
        node = pending.pop()
        removed += 1
        for child in graph.get(node, set()):
            indegree[child] -= 1
            if indegree[child] == 0:
                pending.append(child)
    return bool(nodes) and removed != len(nodes)


def has_corrupt_media(
    values: dict[str, bytes],
    relationships: list[tuple[str, str, str, bool]],
    relationship_sources: set[str],
) -> bool:
    for source, target, relationship_type, external in relationships:
        part = target_part(source, target)
        if (
            external
            or source not in relationship_sources
            or not relationship_type.endswith("/image")
            or part not in values
        ):
            continue
        value = values[part]
        suffix = Path(part).suffix.lower()
        if suffix == ".png" and not value.startswith(b"\x89PNG\r\n\x1a\n"):
            return True
        if suffix in {".jpg", ".jpeg"} and not value.startswith(b"\xff\xd8\xff"):
            return True
        if suffix == ".gif" and not value.startswith((b"GIF87a", b"GIF89a")):
            return True
    return False


def target_part(source: str, target: str) -> str:
    if target.startswith("/"):
        return target.lstrip("/")
    parent = posixpath.dirname(source)
    return posixpath.normpath(posixpath.join(parent, target)).lstrip("/")


def _relationship_source(name: str) -> str:
    if name == "_rels/.rels":
        return ""
    parent, separator, relation_name = name.rpartition("/_rels/")
    if not separator or not relation_name.endswith(".rels"):
        return ""
    return f"{parent}/{relation_name[:-5]}".lstrip("/")


def _contains_entity(value: bytes) -> bool:
    upper = value.upper()
    return b"<!DOCTYPE" in upper and b"<!ENTITY" in upper
