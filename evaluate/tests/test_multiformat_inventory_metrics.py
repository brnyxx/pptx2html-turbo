import json
import tempfile
import unittest
from decimal import Decimal
from pathlib import Path

from evaluate.multiformat_graphemes import split_graphemes
from evaluate.multiformat_inventory import parse_inventory
from evaluate.multiformat_inventory_compare import compare_inventories
from evaluate.multiformat_inventory_types import (
    Box,
    Inventory,
    ObjectItem,
    TextItem,
)
from evaluate.multiformat_metric_types import MetricError


class MultiFormatInventoryMetricTests(unittest.TestCase):
    def test_extended_emoji_sequences_are_single_graphemes(self) -> None:
        self.assertEqual(split_graphemes("🇺🇸"), ["🇺🇸"])
        self.assertEqual(split_graphemes("👍🏽"), ["👍🏽"])
        self.assertEqual(split_graphemes("👩‍💻"), ["👩‍💻"])

    def test_object_occurrences_must_be_contiguous_per_semantic_identity(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            path = Path(temp_dir) / "inventory.json"
            path.write_text(
                json.dumps(
                    {
                        "schema_version": 1,
                        "unit_id": "unit-1",
                        "texts": [],
                        "cells": [],
                        "objects": [
                            {
                                "identity": "image|same|99",
                                "type": "image",
                                "semantic_value": "same",
                                "occurrence": 99,
                                "box": [0, 0, 10, 10],
                            }
                        ],
                    }
                ),
                encoding="utf-8",
            )

            with self.assertRaisesRegex(MetricError, "inventory.identity"):
                parse_inventory(path, "unit-1")

    def test_identical_text_objects_boxes_order_and_baselines_score_100(self) -> None:
        inventory = Inventory(
            unit_id="unit-1",
            texts=(
                TextItem("first", "Alpha", Box(0, 0, 20, 10), 8, 0),
                TextItem("second", "Beta", Box(0, 20, 20, 10), 28, 1),
            ),
            cells=(),
            objects=(ObjectItem("image|abc|1", "image", "abc", 1, Box(30, 0, 10, 10)),),
        )

        result = compare_inventories(inventory, inventory, spreadsheet=False)

        self.assertEqual(result.text_or_cell_similarity, Decimal(100))
        self.assertEqual(result.object_f1, Decimal(100))
        self.assertEqual(result.matched_box_iou, Decimal(100))
        self.assertEqual(result.reading_order_similarity, Decimal(100))
        self.assertEqual(result.baseline_similarity, Decimal(100))

    def test_reversed_reading_order_reduces_content_and_order_scores(self) -> None:
        reference = self._text_inventory(("first", "second"))
        candidate = self._text_inventory(("second", "first"))

        result = compare_inventories(reference, candidate, spreadsheet=False)

        self.assertLess(result.text_or_cell_similarity, Decimal(100))
        self.assertEqual(result.reading_order_similarity, Decimal(0))

    def test_empty_inventories_are_exactly_equal(self) -> None:
        empty = Inventory("empty", (), (), ())

        result = compare_inventories(empty, empty, spreadsheet=False)

        self.assertEqual(result.text_or_cell_similarity, Decimal(100))
        self.assertEqual(result.object_f1, Decimal(100))
        self.assertEqual(result.matched_box_iou, Decimal(100))

    def test_duplicate_objects_use_global_minimum_center_assignment(self) -> None:
        reference = Inventory(
            "unit-1",
            (),
            (),
            (
                ObjectItem("image|same|1", "image", "same", 1, Box(0, 0, 10, 10)),
                ObjectItem("image|same|2", "image", "same", 2, Box(100, 0, 10, 10)),
            ),
        )
        candidate = Inventory(
            "unit-1",
            (),
            (),
            (
                ObjectItem("image|same|1", "image", "same", 1, Box(100, 0, 10, 10)),
                ObjectItem("image|same|2", "image", "same", 2, Box(0, 0, 10, 10)),
            ),
        )

        result = compare_inventories(reference, candidate, spreadsheet=False)

        self.assertEqual(result.object_f1, Decimal(100))
        self.assertEqual(result.matched_box_iou, Decimal(100))

    @staticmethod
    def _text_inventory(order: tuple[str, str]) -> Inventory:
        values = {"first": "Alpha", "second": "Beta"}
        items = tuple(
            TextItem(
                identity,
                values[identity],
                Box(0, index * 20, 20, 10),
                index * 20 + 8,
                index,
            )
            for index, identity in enumerate(order)
        )
        return Inventory("unit-1", items, (), ())
