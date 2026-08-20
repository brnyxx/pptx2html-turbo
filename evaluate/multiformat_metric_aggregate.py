from __future__ import annotations

from decimal import Decimal
from typing import Sequence

from evaluate.multiformat_metric_types import (
    MetricError,
    ScoreSummary,
    UnitScore,
    retained_decimal,
)


def aggregate_units(units: Sequence[UnitScore]) -> ScoreSummary:
    if not units:
        raise MetricError("metrics.units", "must not be empty")
    count = len(units)
    divisor = Decimal(count)
    return ScoreSummary(
        count=count,
        visual=retained_decimal(
            sum((unit.visual for unit in units), start=Decimal(0)) / divisor
        ),
        content=retained_decimal(
            sum((unit.content for unit in units), start=Decimal(0)) / divisor
        ),
        layout=retained_decimal(
            sum((unit.layout for unit in units), start=Decimal(0)) / divisor
        ),
        score=retained_decimal(
            sum((unit.score for unit in units), start=Decimal(0)) / divisor
        ),
        minimum=min(unit.score for unit in units),
    )


def aggregate_files(files: Sequence[Sequence[UnitScore]]) -> ScoreSummary:
    if not files:
        raise MetricError("metrics.files", "must not be empty")
    summaries = [aggregate_units(units) for units in files]
    count = len(summaries)
    divisor = Decimal(count)
    return ScoreSummary(
        count=count,
        visual=retained_decimal(
            sum((item.visual for item in summaries), start=Decimal(0)) / divisor
        ),
        content=retained_decimal(
            sum((item.content for item in summaries), start=Decimal(0)) / divisor
        ),
        layout=retained_decimal(
            sum((item.layout for item in summaries), start=Decimal(0)) / divisor
        ),
        score=retained_decimal(
            sum((item.score for item in summaries), start=Decimal(0)) / divisor
        ),
        minimum=min(item.score for item in summaries),
    )
