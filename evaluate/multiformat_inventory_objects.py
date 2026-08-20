from __future__ import annotations

from collections import Counter, defaultdict
from decimal import Decimal

from evaluate.multiformat_inventory_types import Box, ObjectItem

HUNDRED = Decimal(100)


def object_f1(
    reference: tuple[ObjectItem, ...],
    candidate: tuple[ObjectItem, ...],
) -> Decimal:
    reference_counts = Counter(_key(item) for item in reference)
    candidate_counts = Counter(_key(item) for item in candidate)
    if not reference_counts and not candidate_counts:
        return HUNDRED
    matches = sum(
        min(count, candidate_counts.get(key, 0))
        for key, count in reference_counts.items()
    )
    denominator = len(reference) + len(candidate)
    return Decimal(200 * matches) / Decimal(denominator) if denominator else HUNDRED


def assigned_object_boxes(
    reference: tuple[ObjectItem, ...],
    candidate: tuple[ObjectItem, ...],
) -> list[tuple[Box, Box]]:
    reference_groups: dict[tuple[str, str], list[ObjectItem]] = defaultdict(list)
    candidate_groups: dict[tuple[str, str], list[ObjectItem]] = defaultdict(list)
    for item in reference:
        reference_groups[_key(item)].append(item)
    for item in candidate:
        candidate_groups[_key(item)].append(item)
    result: list[tuple[Box, Box]] = []
    for key in reference_groups.keys() & candidate_groups.keys():
        left = reference_groups[key]
        right = candidate_groups[key]
        costs = [
            [_center_distance(left_item.box, right_item.box) for right_item in right]
            for left_item in left
        ]
        for left_index, right_index in _minimum_assignment(costs):
            result.append((left[left_index].box, right[right_index].box))
    return result


def _minimum_assignment(costs: list[list[float]]) -> list[tuple[int, int]]:
    if not costs or not costs[0]:
        return []
    transposed = len(costs) > len(costs[0])
    matrix = [list(row) for row in zip(*costs, strict=True)] if transposed else costs
    row_count = len(matrix)
    column_count = len(matrix[0])
    row_potential = [0.0] * (row_count + 1)
    column_potential = [0.0] * (column_count + 1)
    matched_row = [0] * (column_count + 1)
    previous_column = [0] * (column_count + 1)
    for row in range(1, row_count + 1):
        matched_row[0] = row
        minimum = [float("inf")] * (column_count + 1)
        used = [False] * (column_count + 1)
        column = 0
        while True:
            used[column] = True
            current_row = matched_row[column]
            delta = float("inf")
            next_column = 0
            for candidate_column in range(1, column_count + 1):
                if used[candidate_column]:
                    continue
                cost = (
                    matrix[current_row - 1][candidate_column - 1]
                    - row_potential[current_row]
                    - column_potential[candidate_column]
                )
                if cost < minimum[candidate_column]:
                    minimum[candidate_column] = cost
                    previous_column[candidate_column] = column
                if minimum[candidate_column] < delta:
                    delta = minimum[candidate_column]
                    next_column = candidate_column
            for candidate_column in range(column_count + 1):
                if used[candidate_column]:
                    row_potential[matched_row[candidate_column]] += delta
                    column_potential[candidate_column] -= delta
                else:
                    minimum[candidate_column] -= delta
            column = next_column
            if matched_row[column] == 0:
                break
        while True:
            previous = previous_column[column]
            matched_row[column] = matched_row[previous]
            column = previous
            if column == 0:
                break
    pairs = [
        (matched_row[column] - 1, column - 1)
        for column in range(1, column_count + 1)
        if matched_row[column] > 0
    ]
    return [(right, left) for left, right in pairs] if transposed else pairs


def _key(item: ObjectItem) -> tuple[str, str]:
    return item.object_type, item.semantic_value


def _center_distance(left: Box, right: Box) -> float:
    left_x = float(left.x) + float(left.width) / 2
    left_y = float(left.y) + float(left.height) / 2
    right_x = float(right.x) + float(right.width) / 2
    right_y = float(right.y) + float(right.height) / 2
    return (left_x - right_x) ** 2 + (left_y - right_y) ** 2
