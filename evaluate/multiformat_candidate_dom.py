from __future__ import annotations

import hashlib
import math
from collections import defaultdict

from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_metric_types import MetricError
from evaluate.multiformat_schema import JsonValue


def inventory_value(
    unit_id: str,
    raw: dict[str, JsonValue],
    document_format: DocumentFormat,
    pixel_scale: float,
) -> dict[str, JsonValue]:
    spreadsheet = document_format in {DocumentFormat.XLS, DocumentFormat.XLSX}
    texts = [] if spreadsheet else _text_values(raw.get("texts"), pixel_scale)
    cells = _cell_values(raw.get("cells"), pixel_scale) if spreadsheet else []
    objects = _object_values(raw.get("objects"), pixel_scale)
    return {
        "schema_version": 1,
        "unit_id": unit_id,
        "texts": texts,
        "cells": cells,
        "objects": objects,
        # The rendered DOM has no notion of an unreproducible number format;
        # attribution refusals originate in the reference extractor.
        "unattributed_cells": [],
    }


def _text_values(value: JsonValue, scale: float) -> list[dict[str, JsonValue]]:
    records = _records(value, "candidate.texts")
    occurrences: defaultdict[str, int] = defaultdict(int)
    result: list[dict[str, JsonValue]] = []
    for record in records:
        text = _string(record, "value")
        semantic = hashlib.sha256(text.encode()).hexdigest()
        occurrences[semantic] += 1
        result.append(
            {
                "identity": f"text|{semantic}|{occurrences[semantic]}",
                "value": text,
                "box": _box(record, scale),
                "baseline": _scaled_number(record.get("baseline"), scale),
                "order": _integer(record, "order"),
            }
        )
    return result


def _cell_values(value: JsonValue, scale: float) -> list[dict[str, JsonValue]]:
    result: list[dict[str, JsonValue]] = []
    for record in _records(value, "candidate.cells"):
        worksheet = _string(record, "worksheet")
        coordinate = _string(record, "coordinate")
        result.append(
            {
                "identity": f"cell|{worksheet}|{coordinate}",
                "worksheet": worksheet,
                "coordinate": coordinate,
                "displayed_value": _string(record, "value"),
                "box": _box(record, scale),
                "baseline": _scaled_number(record.get("baseline"), scale),
                "order": _integer(record, "order"),
            }
        )
    return result


def _object_values(value: JsonValue, scale: float) -> list[dict[str, JsonValue]]:
    records = _records(value, "candidate.objects")
    occurrences: defaultdict[tuple[str, str], int] = defaultdict(int)
    result: list[dict[str, JsonValue]] = []
    for record in records:
        object_type = _string(record, "type")
        raw_semantic = _string(record, "semantic")
        semantic = (
            raw_semantic
            if object_type == "link"
            else hashlib.sha256(raw_semantic.encode()).hexdigest()
        )
        key = object_type, semantic
        occurrences[key] += 1
        occurrence = occurrences[key]
        result.append(
            {
                "identity": f"{object_type}|{semantic}|{occurrence}",
                "type": object_type,
                "semantic_value": semantic,
                "occurrence": occurrence,
                "box": _box(record, scale),
            }
        )
    return result


def _records(value: JsonValue, reason: str) -> list[dict[str, JsonValue]]:
    if not isinstance(value, list) or any(not isinstance(item, dict) for item in value):
        raise MetricError(reason, "expected object list")
    return value


def _box(record: dict[str, JsonValue], scale: float) -> list[JsonValue]:
    value = record.get("box")
    if not isinstance(value, dict):
        raise MetricError("candidate.box", "expected object")
    return [
        _scaled_number(value.get(field), scale)
        for field in ["x", "y", "width", "height"]
    ]


def _scaled_number(value: JsonValue, scale: float) -> float:
    if not isinstance(value, int | float) or isinstance(value, bool):
        raise MetricError("candidate.number", repr(value))
    result = float(value) * scale
    if not math.isfinite(result):
        raise MetricError("candidate.number", repr(value))
    return round(result, 6)


def _string(record: dict[str, JsonValue], field: str) -> str:
    value = record.get(field)
    if not isinstance(value, str) or not value:
        raise MetricError(f"candidate.{field}", repr(value))
    return value


def _integer(record: dict[str, JsonValue], field: str) -> int:
    value = record.get(field)
    if not isinstance(value, int) or isinstance(value, bool) or value < 0:
        raise MetricError(f"candidate.{field}", repr(value))
    return value
