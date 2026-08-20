from __future__ import annotations

from typing import Final

from evaluate.multiformat_pdf import MAX_OBJECTS
from evaluate.multiformat_security_pdf_parser import (
    catalog_id,
    dictionary_view,
)
from evaluate.multiformat_security_pdf_tokens import (
    PdfObjectId,
    direct_array_values,
    direct_array_references,
    direct_dictionary_references,
    is_pdf_string,
    reference_value,
    top_level_integer,
    top_level_name,
    top_level_reference,
    top_level_value,
)

MAX_IMAGE_PIXELS: Final[int] = 100_000_000


def page_ids(objects: dict[PdfObjectId, bytes]) -> frozenset[PdfObjectId]:
    catalog = catalog_id(objects)
    if catalog is None:
        return frozenset()
    pages = top_level_reference(dictionary_view(objects[catalog]), b"Pages")
    if pages is None:
        return frozenset()
    pending = [pages]
    visited: set[PdfObjectId] = set()
    result: set[PdfObjectId] = set()
    while pending and len(visited) <= MAX_OBJECTS:
        object_id = pending.pop()
        if object_id in visited:
            continue
        body = objects.get(object_id)
        if body is None:
            continue
        visited.add(object_id)
        view = dictionary_view(body)
        if top_level_name(view, b"Type") == b"Page":
            result.add(object_id)
            continue
        kids = top_level_value(view, b"Kids")
        if kids is not None:
            children = direct_array_references(kids)
            if children is not None:
                pending.extend(children)
        if len(pending) > MAX_OBJECTS:
            return frozenset()
    return frozenset(result)


def action_ids(
    objects: dict[PdfObjectId, bytes],
) -> frozenset[PdfObjectId]:
    result: set[PdfObjectId] = set()
    catalog = catalog_id(objects)
    if catalog is not None:
        open_action = top_level_reference(
            dictionary_view(objects[catalog]),
            b"OpenAction",
        )
        if open_action is not None:
            result.add(open_action)
    for page_id in page_ids(objects):
        page = dictionary_view(objects[page_id])
        annotations = top_level_value(page, b"Annots")
        if annotations is None:
            continue
        annotation_ids = direct_array_references(annotations)
        if annotation_ids is None:
            continue
        for annotation_id in annotation_ids:
            annotation = objects.get(annotation_id)
            if annotation is None:
                continue
            view = dictionary_view(annotation)
            if top_level_name(view, b"Type") != b"Annot" and top_level_name(
                view,
                b"Subtype",
            ) not in {b"Link", b"Widget"}:
                continue
            action = top_level_reference(view, b"A")
            if action is not None:
                result.add(action)
    return frozenset(result)


def has_embedded_file(objects: dict[PdfObjectId, bytes]) -> bool:
    catalog = catalog_id(objects)
    if catalog is None:
        return False
    names = _dictionary_for_key(
        dictionary_view(objects[catalog]),
        b"Names",
        objects,
    )
    if names is None:
        return False
    embedded = _dictionary_for_key(names, b"EmbeddedFiles", objects)
    if embedded is None:
        return False
    names_array = top_level_value(embedded, b"Names")
    if names_array is None:
        return False
    name_items = direct_array_values(names_array)
    if name_items is None or not name_items or len(name_items) % 2 != 0:
        return False
    file_spec_ids: list[PdfObjectId] = []
    for index in range(0, len(name_items), 2):
        name = name_items[index].strip()
        file_spec_id = reference_value(name_items[index + 1])
        if not is_pdf_string(name) or file_spec_id is None:
            return False
        file_spec_ids.append(file_spec_id)
    for file_spec_id in file_spec_ids:
        file_spec = objects.get(file_spec_id)
        if file_spec is None:
            continue
        file_spec_view = dictionary_view(file_spec)
        if top_level_name(file_spec_view, b"Type") != b"Filespec":
            continue
        embedded_files = _dictionary_for_key(file_spec_view, b"EF", objects)
        embedded_id = (
            top_level_reference(embedded_files, b"F")
            if embedded_files is not None
            else None
        )
        embedded_body = objects.get(embedded_id) if embedded_id is not None else None
        if (
            embedded_body is not None
            and top_level_name(
                dictionary_view(embedded_body),
                b"Type",
            )
            == b"EmbeddedFile"
        ):
            return True
    return False


def has_oversized_page_image(objects: dict[PdfObjectId, bytes]) -> bool:
    for page_id in page_ids(objects):
        resources = _dictionary_for_key(
            dictionary_view(objects[page_id]),
            b"Resources",
            objects,
        )
        if resources is None:
            continue
        xobjects = _dictionary_for_key(resources, b"XObject", objects)
        if xobjects is None:
            continue
        image_ids = direct_dictionary_references(xobjects)
        if image_ids is None:
            continue
        for image_id in image_ids:
            body = objects.get(image_id)
            if body is None:
                continue
            view = dictionary_view(body)
            if top_level_name(view, b"Subtype") != b"Image":
                continue
            width = top_level_integer(view, b"Width")
            height = top_level_integer(view, b"Height")
            if (
                width is not None
                and height is not None
                and width * height > MAX_IMAGE_PIXELS
            ):
                return True
    return False


def _dictionary_for_key(
    value: bytes,
    key: bytes,
    objects: dict[PdfObjectId, bytes],
) -> bytes | None:
    item = top_level_value(value, key)
    if item is None:
        return None
    reference = reference_value(item)
    if reference is not None:
        body = objects.get(reference)
        return dictionary_view(body) if body is not None else None
    return item if item.strip().startswith(b"<<") else None
