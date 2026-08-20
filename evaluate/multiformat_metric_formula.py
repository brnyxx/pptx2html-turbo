from __future__ import annotations

from decimal import Decimal
from typing import Callable

from evaluate.multiformat_metric_types import (
    MetricError,
    PrimitiveValues,
    UnitScore,
    retained_decimal,
)

HUNDRED = Decimal(100)
ZERO = Decimal(0)


def score_unit(
    unit_id: str,
    primitives: PrimitiveValues,
    applicable_metrics: frozenset[str],
) -> UnitScore:
    visual = _component(
        "visual",
        applicable_metrics,
        (
            primitives.ms_ssim,
            primitives.active_tile_ssim,
            primitives.color_similarity,
            primitives.edge_f1,
        ),
        lambda values: (
            Decimal("0.35") * values[0]
            + Decimal("0.25") * values[1]
            + Decimal("0.20") * values[2]
            + Decimal("0.20") * values[3]
        ),
    )
    content = _component(
        "content",
        applicable_metrics,
        (
            primitives.text_or_cell_similarity,
            primitives.object_f1,
        ),
        lambda values: Decimal("0.70") * values[0] + Decimal("0.30") * values[1],
    )
    layout = _component(
        "layout",
        applicable_metrics,
        (
            primitives.matched_box_iou,
            primitives.reading_order_similarity,
            primitives.baseline_similarity,
        ),
        lambda values: (
            Decimal("0.70") * values[0]
            + Decimal("0.30")
            * (Decimal("0.50") * values[1] + Decimal("0.50") * values[2])
        ),
    )
    score = retained_decimal(
        Decimal("0.60") * visual + Decimal("0.25") * content + Decimal("0.15") * layout
    )
    return UnitScore(unit_id, visual, content, layout, score)


def _component(
    name: str,
    applicable_metrics: frozenset[str],
    values: tuple[Decimal | None, ...],
    formula: Callable[[tuple[Decimal, ...]], Decimal],
) -> Decimal:
    if name not in applicable_metrics:
        if any(value is not None for value in values):
            raise MetricError(f"metrics.{name}", "non-applicable evidence is not empty")
        return HUNDRED
    normalized = tuple(
        ZERO if value is None else _bounded(value, name) for value in values
    )
    return retained_decimal(formula(normalized))


def _bounded(value: Decimal, name: str) -> Decimal:
    if not ZERO <= value <= HUNDRED:
        raise MetricError(f"metrics.{name}", str(value))
    return retained_decimal(value)
