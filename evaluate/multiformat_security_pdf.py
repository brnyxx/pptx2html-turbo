from __future__ import annotations

from pathlib import Path

from evaluate.multiformat_package_validation import MAX_SOURCE_BYTES
from evaluate.multiformat_pdf import MAX_OBJECTS, MAX_PAGE_DEPTH
from evaluate.multiformat_security_pdf_graph import (
    action_ids,
    has_embedded_file,
    has_oversized_page_image,
)
from evaluate.multiformat_security_pdf_parser import (
    PdfStructure,
    active_pdf_structure,
    catalog_id,
    dictionary_view,
    fallback_pdf_structure,
    has_xref_cycle,
)
from evaluate.multiformat_security_pdf_tokens import (
    PdfObjectId,
    direct_array_references,
    top_level_integer,
    top_level_name,
    top_level_reference,
    top_level_value,
)


def detect_pdf_security_families(path: Path) -> frozenset[str]:
    try:
        value = path.read_bytes()
    except OSError:
        return frozenset()
    if not value.startswith(b"%PDF-") or len(value) > MAX_SOURCE_BYTES:
        return frozenset()
    active = active_pdf_structure(value)
    if active is None:
        fallback = fallback_pdf_structure(value)
        return (
            frozenset({"malformed-xref"})
            if fallback is not None and _has_coherent_root(fallback)
            else frozenset()
        )
    if not _has_coherent_root(active):
        return frozenset()
    return _detect_active_families(value, active)


def _detect_active_families(
    value: bytes,
    structure: PdfStructure,
) -> frozenset[str]:
    families: set[str] = set()
    objects = structure.objects
    if has_xref_cycle(value, structure):
        families.add("xref-cycle")
    if _has_object_stream_bomb(objects):
        families.add("object-stream-bomb")
    if _has_deep_page_tree(objects):
        families.add("deep-page-tree")
    if has_oversized_page_image(objects):
        families.add("oversized-image")
    if top_level_reference(structure.trailer, b"Encrypt") is not None:
        families.add("encrypted-document")
    if has_embedded_file(objects):
        families.add("embedded-file")
    actions = action_ids(objects)
    if _has_action(objects, actions, b"JavaScript"):
        families.add("javascript-action")
    if _has_action(objects, actions, b"Launch"):
        families.add("launch-action")
    if _has_action(objects, actions, b"URI"):
        families.add("external-uri")
    return frozenset(families)


def _has_coherent_root(structure: PdfStructure) -> bool:
    root = top_level_reference(structure.trailer, b"Root")
    catalog = catalog_id(structure.objects)
    return root is not None and root == catalog


def _has_object_stream_bomb(objects: dict[PdfObjectId, bytes]) -> bool:
    for body in objects.values():
        view = dictionary_view(body)
        if top_level_name(view, b"Type") != b"ObjStm":
            continue
        count = top_level_integer(view, b"N")
        length = top_level_integer(view, b"Length")
        if (
            count is not None
            and count > MAX_OBJECTS
            or length is not None
            and length > MAX_SOURCE_BYTES
        ):
            return True
    return False


def _has_deep_page_tree(objects: dict[PdfObjectId, bytes]) -> bool:
    catalog = catalog_id(objects)
    if catalog is None:
        return False
    pages = top_level_reference(dictionary_view(objects[catalog]), b"Pages")
    if pages is None:
        return False
    pending = [(pages, frozenset(), 0)]
    while pending:
        object_id, ancestors, depth = pending.pop()
        if depth > MAX_PAGE_DEPTH:
            return True
        if object_id in ancestors:
            continue
        body = objects.get(object_id)
        if body is None:
            continue
        view = dictionary_view(body)
        if top_level_name(view, b"Type") == b"Page":
            continue
        kids = top_level_value(view, b"Kids")
        if kids is None:
            continue
        children = direct_array_references(kids)
        if children is None:
            continue
        next_ancestors = frozenset((*ancestors, object_id))
        pending.extend((child, next_ancestors, depth + 1) for child in children)
        if len(pending) > MAX_OBJECTS:
            return False
    return False


def _has_action(
    objects: dict[PdfObjectId, bytes],
    action_ids: frozenset[PdfObjectId],
    action: bytes,
) -> bool:
    return any(
        object_id in objects
        and top_level_name(dictionary_view(objects[object_id]), b"S") == action
        for object_id in action_ids
    )
