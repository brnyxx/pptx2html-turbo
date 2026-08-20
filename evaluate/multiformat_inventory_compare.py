from __future__ import annotations

from decimal import Decimal
from typing import Sequence, TypeVar

from evaluate.multiformat_inventory_types import (
    Box,
    CellItem,
    Inventory,
    InventoryScores,
    MetricNumber,
    TextItem,
)
from evaluate.multiformat_inventory_objects import (
    assigned_object_boxes,
    object_f1,
)
from evaluate.multiformat_graphemes import split_graphemes

HUNDRED = Decimal(100)
ValueT = TypeVar("ValueT")


def compare_inventories(
    reference: Inventory,
    candidate: Inventory,
    *,
    spreadsheet: bool,
) -> InventoryScores:
    reference_ordered = _ordered_items(reference, spreadsheet)
    candidate_ordered = _ordered_items(candidate, spreadsheet)
    content = _content_similarity(reference_ordered, candidate_ordered, spreadsheet)
    objects = object_f1(reference.objects, candidate.objects)
    box_iou = _box_similarity(reference, candidate, spreadsheet)
    reading_order = _reading_order(reference_ordered, candidate_ordered)
    baseline = _baseline_similarity(reference_ordered, candidate_ordered)
    return InventoryScores(content, objects, box_iou, reading_order, baseline)


def _ordered_items(
    inventory: Inventory,
    spreadsheet: bool,
) -> tuple[TextItem | CellItem, ...]:
    values: tuple[TextItem | CellItem, ...] = (
        inventory.cells if spreadsheet else inventory.texts
    )
    return tuple(sorted(values, key=lambda item: item.order))


def _content_similarity(
    reference: Sequence[TextItem | CellItem],
    candidate: Sequence[TextItem | CellItem],
    spreadsheet: bool,
) -> Decimal:
    if spreadsheet:
        reference_values = [_cell_tuple(item) for item in reference]
        candidate_values = [_cell_tuple(item) for item in candidate]
        return _sequence_similarity(reference_values, candidate_values)
    reference_graphemes = split_graphemes(
        "\n".join(_text_value(item) for item in reference)
    )
    candidate_graphemes = split_graphemes(
        "\n".join(_text_value(item) for item in candidate)
    )
    return _sequence_similarity(reference_graphemes, candidate_graphemes)


def _box_similarity(
    reference: Inventory,
    candidate: Inventory,
    spreadsheet: bool,
) -> Decimal:
    reference_ordered = reference.cells if spreadsheet else reference.texts
    candidate_ordered = candidate.cells if spreadsheet else candidate.texts
    reference_boxes = {item.identity: item.box for item in reference_ordered}
    candidate_boxes = {item.identity: item.box for item in candidate_ordered}
    identities = set(reference_boxes) | set(candidate_boxes)
    object_pairs = assigned_object_boxes(reference.objects, candidate.objects)
    denominator = max(
        len(reference_boxes) + len(reference.objects),
        len(candidate_boxes) + len(candidate.objects),
    )
    if denominator == 0:
        return HUNDRED
    total = sum(
        (
            _iou(reference_boxes[identity], candidate_boxes[identity])
            if identity in reference_boxes and identity in candidate_boxes
            else Decimal(0)
            for identity in identities
        ),
        start=Decimal(0),
    )
    total += sum(
        (
            _iou(reference_box, candidate_box)
            for reference_box, candidate_box in object_pairs
        ),
        start=Decimal(0),
    )
    return HUNDRED * total / Decimal(denominator)


def _reading_order(
    reference: Sequence[TextItem | CellItem],
    candidate: Sequence[TextItem | CellItem],
) -> Decimal:
    reference_ids = [item.identity for item in reference]
    candidate_ids = [item.identity for item in candidate]
    if not reference_ids and not candidate_ids:
        return HUNDRED
    if set(reference_ids) != set(candidate_ids):
        return Decimal(0)
    if len(reference_ids) < 2:
        return HUNDRED
    positions = {identity: index for index, identity in enumerate(candidate_ids)}
    sequence = [positions[identity] for identity in reference_ids]
    inversions = sum(
        1
        for index, left in enumerate(sequence)
        for right in sequence[index + 1 :]
        if left > right
    )
    pairs = len(sequence) * (len(sequence) - 1) // 2
    return HUNDRED * (Decimal(1) - Decimal(inversions) / Decimal(pairs))


def _baseline_similarity(
    reference: Sequence[TextItem | CellItem],
    candidate: Sequence[TextItem | CellItem],
) -> Decimal:
    reference_by_id = {item.identity: item for item in reference}
    candidate_by_id = {item.identity: item for item in candidate}
    identities = set(reference_by_id) | set(candidate_by_id)
    if not identities:
        return HUNDRED
    total = Decimal(0)
    for identity in identities:
        if identity not in reference_by_id or identity not in candidate_by_id:
            continue
        expected = reference_by_id[identity]
        actual = candidate_by_id[identity]
        height = max(_decimal(expected.box.height), Decimal("0.000001"))
        error = abs(_decimal(expected.baseline) - _decimal(actual.baseline)) / height
        total += Decimal(1) - min(error, Decimal(1))
    return HUNDRED * total / Decimal(len(identities))


def _iou(left: Box, right: Box) -> Decimal:
    left_x2 = _decimal(left.x) + _decimal(left.width)
    left_y2 = _decimal(left.y) + _decimal(left.height)
    right_x2 = _decimal(right.x) + _decimal(right.width)
    right_y2 = _decimal(right.y) + _decimal(right.height)
    intersection_width = max(
        Decimal(0),
        min(left_x2, right_x2) - max(_decimal(left.x), _decimal(right.x)),
    )
    intersection_height = max(
        Decimal(0),
        min(left_y2, right_y2) - max(_decimal(left.y), _decimal(right.y)),
    )
    intersection = intersection_width * intersection_height
    union = (
        _decimal(left.width) * _decimal(left.height)
        + _decimal(right.width) * _decimal(right.height)
        - intersection
    )
    return intersection / union if union > 0 else Decimal(0)


def _sequence_similarity(
    reference: Sequence[ValueT],
    candidate: Sequence[ValueT],
) -> Decimal:
    maximum = max(len(reference), len(candidate))
    if maximum == 0:
        return HUNDRED
    distance = _edit_distance(reference, candidate)
    return HUNDRED * (Decimal(1) - Decimal(distance) / Decimal(maximum))


def _edit_distance(left: Sequence[ValueT], right: Sequence[ValueT]) -> int:
    previous = list(range(len(right) + 1))
    for left_index, left_value in enumerate(left, start=1):
        current = [left_index]
        for right_index, right_value in enumerate(right, start=1):
            current.append(
                min(
                    current[-1] + 1,
                    previous[right_index] + 1,
                    previous[right_index - 1] + (left_value != right_value),
                )
            )
        previous = current
    return previous[-1]


def _cell_tuple(item: TextItem | CellItem) -> tuple[str, str, str]:
    if isinstance(item, CellItem):
        return item.worksheet, item.coordinate, item.displayed_value
    return "", "", item.value


def _text_value(item: TextItem | CellItem) -> str:
    return item.displayed_value if isinstance(item, CellItem) else item.value


def _decimal(value: MetricNumber) -> Decimal:
    return value if isinstance(value, Decimal) else Decimal(str(value))
