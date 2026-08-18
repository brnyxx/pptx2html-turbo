from __future__ import annotations

import json
from pathlib import Path
from typing import cast
from xml.etree import ElementTree
from zipfile import BadZipFile, ZipFile

from evaluate.adjustment_cases import (
    AdjustmentCase,
    build_adjustment_cases,
    load_adjustment_specs,
)

NAMESPACES = {
    "a": "http://schemas.openxmlformats.org/drawingml/2006/main",
    "p": "http://schemas.openxmlformats.org/presentationml/2006/main",
}


class AdjustmentEvidenceError(RuntimeError):
    pass


def validate_adjustment_corpus(
    canonical_manifest: Path,
    corpus_manifest: Path,
    deck_path: Path,
) -> dict[str, object]:
    specs = load_adjustment_specs(canonical_manifest)
    expected_cases = build_adjustment_cases(specs)
    expected = {
        (case.preset, case.key, case.variant): case
        for case in expected_cases
    }
    payload = _as_object(_load_json(corpus_manifest), "manifest")
    entries = _as_list(payload.get("entries"), "entries")
    if len(entries) != len(expected_cases):
        details = f"expected={len(expected_cases)}:actual={len(entries)}"
        raise AdjustmentEvidenceError(
            f"ADJUSTMENT_CASE_COUNT_MISMATCH:{details}"
        )
    expected_slide_count = _as_int(
        payload.get("slide_count"),
        "slide_count",
    )
    manifest_shapes: dict[str, tuple[AdjustmentCase, dict[str, int]]] = {}
    covered: set[tuple[str, str, str]] = set()
    for raw_entry in entries:
        entry = _as_object(raw_entry, "entry")
        shape_name = _as_str(entry.get("shape_name"), "entry.shape_name")
        key = (
            _as_str(entry.get("preset"), "entry.preset"),
            _as_str(entry.get("key"), "entry.key"),
            _as_str(entry.get("variant"), "entry.variant"),
        )
        case = expected.get(key)
        if case is None or key in covered:
            raise AdjustmentEvidenceError(
                f"ADJUSTMENT_CASE_INVALID:{':'.join(key)}"
            )
        covered.add(key)
        adjustments = _int_map(entry.get("adjustments"), "entry.adjustments")
        value = _as_int(entry.get("value"), "entry.value")
        range_verification = _as_str(
            entry.get("range_verification"),
            "entry.range_verification",
        )
        if (
            value != case.value
            or adjustments != case.adjustments
            or range_verification != case.range_verification
        ):
            raise AdjustmentEvidenceError(
                f"ADJUSTMENT_CASE_VALUE_MISMATCH:{shape_name}"
            )
        if shape_name in manifest_shapes:
            raise AdjustmentEvidenceError(
                f"ADJUSTMENT_SHAPE_DUPLICATE:{shape_name}"
            )
        manifest_shapes[shape_name] = (case, adjustments)
    if covered != set(expected):
        raise AdjustmentEvidenceError("ADJUSTMENT_CASE_INVENTORY_MISMATCH")

    actual_shapes, actual_slide_count = _ooxml_shapes(deck_path)
    if actual_slide_count != expected_slide_count:
        details = f"expected={expected_slide_count}:actual={actual_slide_count}"
        raise AdjustmentEvidenceError(
            f"ADJUSTMENT_SLIDE_COUNT_MISMATCH:{details}"
        )
    if set(actual_shapes) != set(manifest_shapes):
        raise AdjustmentEvidenceError("ADJUSTMENT_OOXML_SHAPE_INVENTORY_MISMATCH")
    for shape_name, (case, adjustments) in manifest_shapes.items():
        preset, actual_adjustments = actual_shapes[shape_name]
        if preset != case.preset or actual_adjustments != adjustments:
            raise AdjustmentEvidenceError(
                f"ADJUSTMENT_OOXML_MISMATCH:{shape_name}"
            )
    return {
        "ok": True,
        "adjustment_pair_count": len(specs),
        "case_count": len(expected_cases),
        "slide_count": expected_slide_count,
        "ooxml_shapes_verified": len(actual_shapes),
    }


def _ooxml_shapes(
    deck_path: Path,
) -> tuple[dict[str, tuple[str, dict[str, int]]], int]:
    result: dict[str, tuple[str, dict[str, int]]] = {}
    try:
        with ZipFile(deck_path) as archive:
            corrupt_part = archive.testzip()
            if corrupt_part is not None:
                raise AdjustmentEvidenceError(
                    f"ADJUSTMENT_DECK_INVALID:{deck_path}:{corrupt_part}"
                )
            slide_names = [
                name
                for name in archive.namelist()
                if name.startswith("ppt/slides/slide")
                and name.endswith(".xml")
            ]
            for slide_name in slide_names:
                root = ElementTree.fromstring(archive.read(slide_name))
                shapes = [
                    (shape, "./p:nvSpPr/p:cNvPr")
                    for shape in root.findall(".//p:sp", NAMESPACES)
                ]
                shapes.extend(
                    (shape, "./p:nvCxnSpPr/p:cNvPr")
                    for shape in root.findall(".//p:cxnSp", NAMESPACES)
                )
                for shape, properties_path in shapes:
                    properties = shape.find(properties_path, NAMESPACES)
                    if properties is None:
                        continue
                    shape_name = properties.get("name", "")
                    if not shape_name.startswith("ADJ_"):
                        continue
                    geometry = shape.find(
                        "./p:spPr/a:prstGeom",
                        NAMESPACES,
                    )
                    if geometry is None or shape_name in result:
                        raise AdjustmentEvidenceError(
                            f"ADJUSTMENT_OOXML_SHAPE_INVALID:{shape_name}"
                        )
                    adjustments = {
                        _required_attribute(guide, "name"): _guide_value(guide)
                        for guide in geometry.findall(
                            "./a:avLst/a:gd",
                            NAMESPACES,
                        )
                    }
                    result[shape_name] = (
                        _required_attribute(geometry, "prst"),
                        adjustments,
                    )
    except (BadZipFile, ElementTree.ParseError, OSError) as error:
        raise AdjustmentEvidenceError(
            f"ADJUSTMENT_DECK_INVALID:{deck_path}:{error}"
        ) from error
    return result, len(slide_names)


def _guide_value(guide: ElementTree.Element) -> int:
    formula = _required_attribute(guide, "fmla")
    if not formula.startswith("val "):
        raise AdjustmentEvidenceError(
            f"ADJUSTMENT_GUIDE_FORMULA_INVALID:{formula}"
        )
    try:
        return int(formula.removeprefix("val "))
    except ValueError as error:
        raise AdjustmentEvidenceError(
            f"ADJUSTMENT_GUIDE_FORMULA_INVALID:{formula}"
        ) from error


def _required_attribute(element: ElementTree.Element, name: str) -> str:
    value = element.get(name)
    if not value:
        raise AdjustmentEvidenceError(
            f"ADJUSTMENT_ATTRIBUTE_MISSING:{name}"
        )
    return value


def _load_json(path: Path) -> object:
    try:
        return cast(object, json.loads(path.read_text(encoding="utf-8")))
    except (OSError, json.JSONDecodeError) as error:
        raise AdjustmentEvidenceError(
            f"ADJUSTMENT_JSON_INVALID:{path}:{error}"
        ) from error


def _as_object(value: object, field: str) -> dict[str, object]:
    if not isinstance(value, dict):
        raise AdjustmentEvidenceError(f"ADJUSTMENT_FIELD_INVALID:{field}")
    return cast(dict[str, object], value)


def _as_list(value: object, field: str) -> list[object]:
    if not isinstance(value, list):
        raise AdjustmentEvidenceError(f"ADJUSTMENT_FIELD_INVALID:{field}")
    return cast(list[object], value)


def _as_str(value: object, field: str) -> str:
    if not isinstance(value, str) or not value:
        raise AdjustmentEvidenceError(f"ADJUSTMENT_FIELD_INVALID:{field}")
    return value


def _as_int(value: object, field: str) -> int:
    if not isinstance(value, int) or isinstance(value, bool):
        raise AdjustmentEvidenceError(f"ADJUSTMENT_FIELD_INVALID:{field}")
    return value


def _int_map(value: object, field: str) -> dict[str, int]:
    payload = _as_object(value, field)
    return {
        _as_str(key, f"{field}.key"): _as_int(item, f"{field}.{key}")
        for key, item in payload.items()
    }
