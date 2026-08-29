from __future__ import annotations

import hashlib
import math
from collections import defaultdict
from pathlib import Path

from evaluate.multiformat_candidate_artifacts import write_canonical_json
from evaluate.multiformat_candidate_browser_checks import (
    AggregateGeometry,
    canonical_office_page_dimension,
)
from evaluate.multiformat_corpus_items import object_list
from evaluate.multiformat_office_oracle_batch import (
    OfficeOracleBatchFile,
)
from evaluate.multiformat_office_oracle_layout import (
    LayoutPage,
    OfficeOracleInventoryError,
    layout_pages,
)
from evaluate.multiformat_schema import JsonValue, string_value
from evaluate.multiformat_strict_json import read_strict_object


def write_office_oracle_inventories(
    source: OfficeOracleBatchFile,
    unit_ids: list[str],
    output_dir: Path,
    *,
    aggregate_pages: bool = False,
    aggregate_geometry: AggregateGeometry | None = None,
) -> list[Path]:
    pages = layout_pages(source.layout)
    if len(pages) != len(source.units) or (
        len(unit_ids) != 1 if aggregate_pages else len(unit_ids) != len(pages)
    ):
        raise OfficeOracleInventoryError("office layout page count differs")
    output_dir.mkdir(parents=True, exist_ok=False)
    spreadsheet = source.document_format in {"xls", "xlsx"}
    cells = (
        _spreadsheet_cells(source.semantic, pages)
        if spreadsheet
        else [[] for _ in pages]
    )
    # Cells whose display text the extractor refused to reproduce are carried
    # through as explicit evidence, so a skipped attribution is never silent.
    unattributed = _unattributed_cells(source.semantic) if spreadsheet else []
    if aggregate_pages:
        if (
            aggregate_geometry is None
            or not math.isfinite(aggregate_geometry.scale)
            or not 0 < aggregate_geometry.scale <= 1
            or len(aggregate_geometry.pages) != len(pages)
        ):
            raise OfficeOracleInventoryError("aggregate geometry is invalid")
        return [
            _write_aggregate_inventory(
                output_dir / "unit-1.json",
                unit_ids[0],
                source,
                pages,
                cells,
                unattributed,
                aggregate_geometry,
            )
        ]
    if aggregate_geometry is not None:
        raise OfficeOracleInventoryError("aggregate geometry is unexpected")
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
        inventory: dict[str, JsonValue] = {
            "schema_version": 1,
            "unit_id": unit_id,
            "texts": list(texts),
            "cells": list(_scaled_cells(cells[index], scale_x, scale_y)),
            "objects": [],
            # Repeated per unit so any single inventory carries the proof.
            "unattributed_cells": unattributed,
        }
        write_canonical_json(path, inventory)
        result.append(path)
    return result


def _write_aggregate_inventory(
    path: Path,
    unit_id: str,
    source: OfficeOracleBatchFile,
    pages: list[LayoutPage],
    cells: list[list[dict[str, JsonValue]]],
    unattributed: list[JsonValue],
    geometry: AggregateGeometry,
) -> Path:
    texts: list[dict[str, JsonValue]] = []
    scaled_cells: list[dict[str, JsonValue]] = []
    occurrences: defaultdict[str, int] = defaultdict(int)
    order = 1
    for page, unit, page_cells, page_geometry in zip(
        pages,
        source.units,
        cells,
        geometry.pages,
        strict=True,
    ):
        page_width = canonical_office_page_dimension(page.width)
        page_height = canonical_office_page_dimension(page.height)
        if (page_width, page_height) != (page_geometry.width, page_geometry.height):
            raise OfficeOracleInventoryError("aggregate page geometry differs")
        scale_x = page_width * geometry.scale / page.width
        scale_y = page_height * geometry.scale / page.height
        offset_y = page_geometry.top * geometry.scale
        if source.document_format not in {"xls", "xlsx"}:
            page_texts = _text_items(
                page,
                scale_x,
                scale_y,
                occurrences,
                order,
                offset_y,
            )
            texts.extend(page_texts)
            order += len(page_texts)
        page_scaled_cells = _scaled_cells(
            page_cells,
            scale_x,
            scale_y,
            order,
            offset_y,
        )
        scaled_cells.extend(page_scaled_cells)
        order += len(page_scaled_cells)
    inventory: dict[str, JsonValue] = {
        "schema_version": 1,
        "unit_id": unit_id,
        "texts": texts,
        "cells": scaled_cells,
        "objects": [],
        "unattributed_cells": unattributed,
    }
    write_canonical_json(path, inventory)
    return path


def _text_items(
    page: LayoutPage,
    scale_x: float,
    scale_y: float,
    occurrences: defaultdict[str, int] | None = None,
    order_start: int = 1,
    offset_y: float = 0,
) -> list[dict[str, JsonValue]]:
    occurrences = occurrences if occurrences is not None else defaultdict(int)
    result: list[dict[str, JsonValue]] = []
    for order, line in enumerate(page.lines, start=order_start):
        semantic = hashlib.sha256(line.value.encode()).hexdigest()
        occurrences[semantic] += 1
        result.append(
            {
                "identity": f"text|{semantic}|{occurrences[semantic]}",
                "value": line.value,
                "box": _scale_box(line.box, scale_x, scale_y, offset_y),
                "baseline": line.baseline * scale_y + offset_y,
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
            record: dict[str, JsonValue] = {
                "identity": f"cell|{name}|{coordinate}",
                "worksheet": name,
                "coordinate": coordinate,
                "displayed_value": display,
                "box": [float(item) for item in line.box],
                "baseline": line.baseline,
            }
            result[page_index].append(record)
    return result


def _unattributed_cells(semantic_path: Path) -> list[JsonValue]:
    """Collects the extractor's structured attribution refusals."""
    semantic = read_strict_object(semantic_path)
    result: list[JsonValue] = []
    for worksheet in object_list(
        semantic,
        "worksheets",
        "office.semantic.worksheets",
    ):
        if worksheet.get("unattributed_cells") is None:
            continue
        for item in object_list(
            worksheet,
            "unattributed_cells",
            "office.semantic.unattributed_cells",
        ):
            refusal: dict[str, JsonValue] = {
                "worksheet": string_value(item, "worksheet"),
                "address": string_value(item, "address"),
                "number_format": string_value(item, "number_format"),
                "reason": string_value(item, "reason"),
            }
            result.append(refusal)
    return result


def _scaled_cells(
    values: list[dict[str, JsonValue]],
    scale_x: float,
    scale_y: float,
    order_start: int = 1,
    offset_y: float = 0,
) -> list[dict[str, JsonValue]]:
    result: list[dict[str, JsonValue]] = []
    for order, value in enumerate(values, start=order_start):
        baseline = value["baseline"]
        if not isinstance(baseline, int | float) or isinstance(baseline, bool):
            raise OfficeOracleInventoryError("office cell geometry is invalid")
        scaled: dict[str, JsonValue] = {
            **value,
            "box": _scale_json_box(value["box"], scale_x, scale_y, offset_y),
            "baseline": baseline * scale_y + offset_y,
            "order": order,
        }
        result.append(scaled)
    return result


def _scale_json_box(
    value: JsonValue,
    scale_x: float,
    scale_y: float,
    offset_y: float = 0,
) -> list[JsonValue]:
    if not isinstance(value, list):
        raise OfficeOracleInventoryError("office cell geometry is invalid")
    box: list[float] = []
    for item in value:
        if not isinstance(item, int | float) or isinstance(item, bool):
            raise OfficeOracleInventoryError("office cell geometry is invalid")
        box.append(float(item))
    return _scale_box(tuple(box), scale_x, scale_y, offset_y)


def _scale_box(
    box: tuple[float, ...],
    scale_x: float,
    scale_y: float,
    offset_y: float = 0,
) -> list[JsonValue]:
    if len(box) != 4:
        raise OfficeOracleInventoryError("office box is invalid")
    return [
        box[0] * scale_x,
        box[1] * scale_y + offset_y,
        box[2] * scale_x,
        box[3] * scale_y,
    ]


__all__ = ["OfficeOracleInventoryError", "write_office_oracle_inventories"]
