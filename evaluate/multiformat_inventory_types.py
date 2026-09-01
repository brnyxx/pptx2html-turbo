from __future__ import annotations

from dataclasses import dataclass
from decimal import Decimal
from typing import TypeAlias

MetricNumber: TypeAlias = int | float | Decimal


@dataclass(frozen=True, slots=True)
class Box:
    x: MetricNumber
    y: MetricNumber
    width: MetricNumber
    height: MetricNumber


@dataclass(frozen=True, slots=True)
class TextItem:
    identity: str
    value: str
    box: Box
    baseline: MetricNumber
    order: int


@dataclass(frozen=True, slots=True)
class CellItem:
    identity: str
    worksheet: str
    coordinate: str
    displayed_value: str
    box: Box
    baseline: MetricNumber
    order: int


@dataclass(frozen=True, slots=True)
class ObjectItem:
    identity: str
    object_type: str
    semantic_value: str
    occurrence: int
    box: Box


@dataclass(frozen=True, slots=True)
class UnattributedCell:
    """A cell deliberately excluded from coordinate attribution.

    Carrying these through the inventory keeps a skipped attribution provable
    rather than indistinguishable from a cell that never existed.
    """

    worksheet: str
    address: str
    number_format: str
    reason: str


@dataclass(frozen=True, slots=True)
class Inventory:
    unit_id: str
    texts: tuple[TextItem, ...]
    cells: tuple[CellItem, ...]
    objects: tuple[ObjectItem, ...]
    unattributed_cells: tuple[UnattributedCell, ...] = ()


@dataclass(frozen=True, slots=True)
class InventoryScores:
    text_or_cell_similarity: Decimal
    object_f1: Decimal
    matched_box_iou: Decimal
    reading_order_similarity: Decimal
    baseline_similarity: Decimal
