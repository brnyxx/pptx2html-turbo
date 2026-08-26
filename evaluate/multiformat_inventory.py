from __future__ import annotations

from decimal import Decimal, InvalidOperation
from pathlib import Path

from evaluate.multiformat_corpus_items import object_list, require_keys
from evaluate.multiformat_corpus_types import CorpusError
from evaluate.multiformat_inventory_types import (
    Box,
    CellItem,
    Inventory,
    ObjectItem,
    TextItem,
    UnattributedCell,
)
from evaluate.multiformat_metric_types import MetricError
from evaluate.multiformat_schema import (
    JsonValue,
    integer_value,
    string_value,
)
from evaluate.multiformat_strict_json import StrictJsonError, read_strict_object


def parse_inventory(path: Path, expected_unit_id: str) -> Inventory:
    try:
        values = read_strict_object(path)
        require_keys(
            values,
            {
                "schema_version",
                "unit_id",
                "texts",
                "cells",
                "objects",
                "unattributed_cells",
            },
            "inventory.schema",
        )
        if integer_value(values, "schema_version") != 1:
            raise MetricError("inventory.schema", "expected version 1")
        unit_id = string_value(values, "unit_id")
        if unit_id != expected_unit_id:
            raise MetricError("inventory.unit_id", unit_id)
        texts = tuple(_parse_texts(values))
        cells = tuple(_parse_cells(values))
        objects = tuple(_parse_objects(values))
        _require_unique(
            [item.identity for item in (*texts, *cells, *objects)],
            "inventory.identity",
        )
        _require_unique(
            [item.order for item in (*texts, *cells)],
            "inventory.order",
        )
        return Inventory(
            unit_id,
            texts,
            cells,
            objects,
            tuple(_parse_unattributed_cells(values)),
        )
    except MetricError:
        raise
    except (
        StrictJsonError,
        CorpusError,
        TypeError,
        ValueError,
        InvalidOperation,
    ) as error:
        raise MetricError("inventory.schema", path.as_posix()) from error


def inventory_boxes(inventory: Inventory) -> tuple[Box, ...]:
    return tuple(
        [
            *(item.box for item in inventory.texts),
            *(item.box for item in inventory.cells),
            *(item.box for item in inventory.objects),
        ]
    )


def _parse_texts(values: dict[str, JsonValue]) -> list[TextItem]:
    result: list[TextItem] = []
    for item in object_list(values, "texts", "inventory.texts"):
        require_keys(
            item,
            {"identity", "value", "box", "baseline", "order"},
            "inventory.text",
        )
        result.append(
            TextItem(
                string_value(item, "identity"),
                string_value(item, "value"),
                _parse_box(item),
                _decimal(item, "baseline"),
                integer_value(item, "order"),
            )
        )
    return result


def _parse_cells(values: dict[str, JsonValue]) -> list[CellItem]:
    result: list[CellItem] = []
    for item in object_list(values, "cells", "inventory.cells"):
        require_keys(
            item,
            {
                "identity",
                "worksheet",
                "coordinate",
                "displayed_value",
                "box",
                "baseline",
                "order",
            },
            "inventory.cell",
        )
        result.append(
            CellItem(
                string_value(item, "identity"),
                string_value(item, "worksheet"),
                string_value(item, "coordinate"),
                string_value(item, "displayed_value"),
                _parse_box(item),
                _decimal(item, "baseline"),
                integer_value(item, "order"),
            )
        )
    return result


def _parse_unattributed_cells(
    values: dict[str, JsonValue],
) -> list[UnattributedCell]:
    result: list[UnattributedCell] = []
    for item in object_list(
        values,
        "unattributed_cells",
        "inventory.unattributed_cells",
    ):
        require_keys(
            item,
            {"worksheet", "address", "number_format", "reason"},
            "inventory.unattributed_cell",
        )
        result.append(
            UnattributedCell(
                string_value(item, "worksheet"),
                string_value(item, "address"),
                string_value(item, "number_format"),
                string_value(item, "reason"),
            )
        )
    return result


def _parse_objects(values: dict[str, JsonValue]) -> list[ObjectItem]:
    result: list[ObjectItem] = []
    for item in object_list(values, "objects", "inventory.objects"):
        require_keys(
            item,
            {
                "identity",
                "type",
                "semantic_value",
                "occurrence",
                "box",
            },
            "inventory.object",
        )
        object_type = string_value(item, "type")
        semantic_value = string_value(item, "semantic_value")
        occurrence = integer_value(item, "occurrence")
        identity = string_value(item, "identity")
        if occurrence <= 0 or identity != (
            f"{object_type}|{semantic_value}|{occurrence}"
        ):
            raise MetricError("inventory.identity", identity)
        result.append(
            ObjectItem(
                identity,
                object_type,
                semantic_value,
                occurrence,
                _parse_box(item),
            )
        )
    occurrences: dict[tuple[str, str], list[int]] = {}
    for item in result:
        occurrences.setdefault(
            (item.object_type, item.semantic_value),
            [],
        ).append(item.occurrence)
    if any(
        sorted(values) != list(range(1, len(values) + 1))
        for values in occurrences.values()
    ):
        raise MetricError("inventory.identity", "non-contiguous occurrence")
    return result


def _parse_box(values: dict[str, JsonValue]) -> Box:
    value = values.get("box")
    if not isinstance(value, list) or len(value) != 4:
        raise MetricError("inventory.box", "expected [x, y, width, height]")
    numbers = [_decimal_value(item) for item in value]
    if numbers[2] < 0 or numbers[3] < 0:
        raise MetricError("inventory.box", "negative extent")
    return Box(*numbers)


def _decimal(values: dict[str, JsonValue], field: str) -> Decimal:
    return _decimal_value(values.get(field))


def _decimal_value(value: JsonValue) -> Decimal:
    if not isinstance(value, int | float | str) or isinstance(value, bool):
        raise MetricError("inventory.number", repr(value))
    result = Decimal(str(value))
    if not result.is_finite():
        raise MetricError("inventory.number", str(value))
    return result


def _require_unique(values: list[str] | list[int], reason: str) -> None:
    if len(values) != len(set(values)):
        raise MetricError(reason, "duplicate value")
