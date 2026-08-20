import unittest
from decimal import Decimal

from evaluate.multiformat_metric_aggregate import (
    aggregate_files,
    aggregate_units,
)
from evaluate.multiformat_metric_formula import score_unit
from evaluate.multiformat_metric_types import PrimitiveValues, UnitScore


class MultiFormatMetricFormulaTests(unittest.TestCase):
    def test_fixed_primitive_weights_produce_exact_unit_components(self) -> None:
        primitives = PrimitiveValues(
            ms_ssim=Decimal(80),
            active_tile_ssim=Decimal(60),
            color_similarity=Decimal(40),
            edge_f1=Decimal(20),
            text_or_cell_similarity=Decimal(90),
            object_f1=Decimal(50),
            matched_box_iou=Decimal(70),
            reading_order_similarity=Decimal(50),
            baseline_similarity=Decimal(30),
        )

        result = score_unit(
            "unit-1",
            primitives,
            frozenset({"visual", "content", "layout"}),
        )

        self.assertEqual(result.visual, Decimal("55.00"))
        self.assertEqual(result.content, Decimal("78.0"))
        self.assertEqual(result.layout, Decimal("61.00"))
        self.assertEqual(result.score, Decimal("61.6500"))

    def test_non_applicable_components_require_empty_primitives_and_score_100(
        self,
    ) -> None:
        primitives = PrimitiveValues(
            ms_ssim=Decimal(100),
            active_tile_ssim=Decimal(100),
            color_similarity=Decimal(100),
            edge_f1=Decimal(100),
            text_or_cell_similarity=None,
            object_f1=None,
            matched_box_iou=None,
            reading_order_similarity=None,
            baseline_similarity=None,
        )

        result = score_unit("unit-1", primitives, frozenset({"visual"}))

        self.assertEqual(result.visual, Decimal(100))
        self.assertEqual(result.content, Decimal(100))
        self.assertEqual(result.layout, Decimal(100))
        self.assertEqual(result.score, Decimal(100))

    def test_blind_aggregation_uses_file_means_before_format_mean(self) -> None:
        high = self._unit("high", 100)
        low = self._unit("low", 0)

        result = aggregate_files([[high, high], [low]])

        self.assertEqual(result.score, Decimal(50))
        self.assertNotEqual(result.score, Decimal(200) / Decimal(3))

    def test_unit_aggregation_retains_exactly_six_decimal_places(self) -> None:
        first = self._unit("first", Decimal("95.9999994"))
        second = self._unit("second", Decimal("96.0000004"))

        result = aggregate_units([first, second])

        self.assertEqual(result.score, Decimal("96.000000"))
        self.assertEqual(result.rounded()["score"], 96.0)

    @staticmethod
    def _unit(unit_id: str, score: int | Decimal) -> UnitScore:
        value = Decimal(score)
        return UnitScore(
            unit_id=unit_id,
            visual=value,
            content=value,
            layout=value,
            score=value,
        )
