from __future__ import annotations

import hashlib
import math
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path
from xml.etree import ElementTree

from evaluate.multiformat_candidate_artifacts import write_canonical_json
from evaluate.multiformat_corpus_items import object_list
from evaluate.multiformat_office_oracle_batch import (
    OfficeOracleBatchFile,
)
from evaluate.multiformat_schema import JsonValue, string_value
from evaluate.multiformat_strict_json import read_strict_object


class OfficeOracleInventoryError(Exception):
    pass


@dataclass(frozen=True, slots=True)
class LayoutLine:
    value: str
    box: tuple[float, float, float, float]
    baseline: float


@dataclass(frozen=True, slots=True)
class LayoutPage:
    width: float
    height: float
    lines: tuple[LayoutLine, ...]


def write_office_oracle_inventories(
    source: OfficeOracleBatchFile,
    unit_ids: list[str],
    output_dir: Path,
) -> list[Path]:
    pages = _layout_pages(source.layout)
    if len(pages) != len(source.units) or len(unit_ids) != len(pages):
        raise OfficeOracleInventoryError("office layout page count differs")
    output_dir.mkdir(parents=True, exist_ok=False)
    cells = (
        _spreadsheet_cells(source.semantic, pages)
        if source.document_format in {"xls", "xlsx"}
        else [[] for _ in pages]
    )
    result: list[Path] = []
    for index, (unit_id, page, unit) in enumerate(
        zip(unit_ids, pages, source.units, strict=True)
    ):
        scale_x = unit.width / page.width
        scale_y = unit.height / page.height
        texts = (
            []
            if source.document_format in {"xls", "xlsx"}
            else _text_items(page, scale_x, scale_y)
        )
        path = output_dir / f"unit-{index + 1}.json"
        write_canonical_json(
            path,
            {
                "schema_version": 1,
                "unit_id": unit_id,
                "texts": texts,
                "cells": _scaled_cells(cells[index], scale_x, scale_y),
                "objects": [],
            },
        )
        result.append(path)
    return result


def _layout_pages(path: Path) -> list[LayoutPage]:
    value = path.read_bytes()
    upper = value.upper()
    if b"<!DOCTYPE" in upper or b"<!ENTITY" in upper:
        raise OfficeOracleInventoryError("office layout XML is unsafe")
    try:
        root = ElementTree.fromstring(value)
    except ElementTree.ParseError as error:
        raise OfficeOracleInventoryError("office layout XML is invalid") from error
    result: list[LayoutPage] = []
    for page in (item for item in root.iter() if _local_name(item.tag) == "page"):
        width = _attribute(page, "width")
        height = _attribute(page, "height")
        if width <= 0 or height <= 0:
            raise OfficeOracleInventoryError("office layout page is invalid")
        lines = tuple(
            parsed
            for item in page.iter()
            if _local_name(item.tag) == "line"
            and (parsed := _layout_line(item)) is not None
        )
        result.append(LayoutPage(width, height, lines))
    if not result:
        raise OfficeOracleInventoryError("office layout has no pages")
    return result


def _layout_line(element: ElementTree.Element) -> LayoutLine | None:
    words = [
        item
        for item in element.iter()
        if _local_name(item.tag) == "word" and (item.text or "").strip()
    ]
    if not words:
        return None
    value = " ".join((item.text or "").strip() for item in words)
    x_min = min(_attribute(item, "xMin") for item in words)
    y_min = min(_attribute(item, "yMin") for item in words)
    x_max = max(_attribute(item, "xMax") for item in words)
    y_max = max(_attribute(item, "yMax") for item in words)
    return LayoutLine(
        value,
        (x_min, y_min, x_max - x_min, y_max - y_min),
        y_max,
    )


def _text_items(
    page: LayoutPage,
    scale_x: float,
    scale_y: float,
) -> list[dict[str, JsonValue]]:
    occurrences: defaultdict[str, int] = defaultdict(int)
    result: list[dict[str, JsonValue]] = []
    for order, line in enumerate(page.lines, start=1):
        semantic = hashlib.sha256(line.value.encode()).hexdigest()
        occurrences[semantic] += 1
        result.append(
            {
                "identity": f"text|{semantic}|{occurrences[semantic]}",
                "value": line.value,
                "box": _scale_box(line.box, scale_x, scale_y),
                "baseline": line.baseline * scale_y,
                "order": order,
            }
        )
    return result


def _spreadsheet_cells(
    semantic_path: Path,
    pages: list[LayoutPage],
) -> list[list[dict[str, JsonValue]]]:
    semantic = read_strict_object(semantic_path)
    result: list[list[dict[str, JsonValue]]] = [[] for _ in pages]
    available = [
        (page_index, line_index, line)
        for page_index, page in enumerate(pages)
        for line_index, line in enumerate(page.lines)
    ]
    used: set[tuple[int, int]] = set()
    for worksheet in object_list(
        semantic,
        "worksheets",
        "office.semantic.worksheets",
    ):
        name = string_value(worksheet, "name")
        for cell in object_list(
            worksheet,
            "cells",
            "office.semantic.cells",
        ):
            display = string_value(cell, "display")
            if not display:
                continue
            match = next(
                (
                    item
                    for item in available
                    if item[:2] not in used and item[2].value == display
                ),
                None,
            )
            if match is None:
                continue
            page_index, line_index, line = match
            used.add((page_index, line_index))
            coordinate = string_value(cell, "address").replace("$", "")
            result[page_index].append(
                {
                    "identity": f"cell|{name}|{coordinate}",
                    "worksheet": name,
                    "coordinate": coordinate,
                    "displayed_value": display,
                    "box": list(line.box),
                    "baseline": line.baseline,
                }
            )
    return result


def _scaled_cells(
    values: list[dict[str, JsonValue]],
    scale_x: float,
    scale_y: float,
) -> list[dict[str, JsonValue]]:
    result: list[dict[str, JsonValue]] = []
    for order, value in enumerate(values, start=1):
        box = value["box"]
        baseline = value["baseline"]
        if not isinstance(box, list) or not isinstance(baseline, int | float):
            raise OfficeOracleInventoryError("office cell geometry is invalid")
        result.append(
            {
                **value,
                "box": _scale_box(tuple(float(item) for item in box), scale_x, scale_y),
                "baseline": baseline * scale_y,
                "order": order,
            }
        )
    return result


def _scale_box(
    box: tuple[float, ...],
    scale_x: float,
    scale_y: float,
) -> list[float]:
    if len(box) != 4:
        raise OfficeOracleInventoryError("office box is invalid")
    return [
        box[0] * scale_x,
        box[1] * scale_y,
        box[2] * scale_x,
        box[3] * scale_y,
    ]


def _attribute(element: ElementTree.Element, name: str) -> float:
    try:
        value = float(element.attrib[name])
    except (KeyError, ValueError) as error:
        raise OfficeOracleInventoryError(
            "office layout attribute is invalid"
        ) from error
    if not math.isfinite(value):
        raise OfficeOracleInventoryError("office layout attribute is non-finite")
    return value


def _local_name(value: str) -> str:
    return value.rsplit("}", 1)[-1]
