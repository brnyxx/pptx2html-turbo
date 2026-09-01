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
from evaluate.multiformat_metric_compute import compute_unit
from evaluate.multiformat_metric_types import MetricError
from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.tests.multiformat_metric_artifact_fixture import (
    binding,
    write_unit_artifacts,
)


class MultiFormatInventoryMetricTests(unittest.TestCase):
    def test_extended_emoji_sequences_are_single_graphemes(self) -> None:
        self.assertEqual(split_graphemes("🇺🇸"), ["🇺🇸"])
        self.assertEqual(split_graphemes("👍🏽"), ["👍🏽"])
        self.assertEqual(split_graphemes("👩‍💻"), ["👩‍💻"])

    def test_unattributed_cells_are_parsed_as_structured_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            path = Path(temp_dir) / "inventory.json"
            path.write_text(
                json.dumps(
                    {
                        "schema_version": 1,
                        "unit_id": "unit-1",
                        "texts": [],
                        "cells": [],
                        "objects": [],
                        "unattributed_cells": [
                            {
                                "worksheet": "Sheet1",
                                "address": "A1",
                                "number_format": "unsupported",
                                "reason": "number format display text is not"
                                " reproduced",
                            }
                        ],
                    }
                ),
                encoding="utf-8",
            )

            inventory = parse_inventory(path, "unit-1")

            # The refusal survives parsing with its identifying detail intact,
            # so downstream gates can act on it.
            self.assertEqual(len(inventory.unattributed_cells), 1)
            self.assertEqual(inventory.unattributed_cells[0].worksheet, "Sheet1")
            self.assertEqual(inventory.unattributed_cells[0].address, "A1")
            self.assertEqual(
                inventory.unattributed_cells[0].number_format, "unsupported"
            )

    def test_unattributable_number_format_fails_the_content_gate(self) -> None:
        """A structured refusal must hard-fail metrics, not just ride along.

        If the diagnostic were accepted-and-ignored, the content metric would
        silently score an incomplete cell set and could still publish READY.
        """
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            artifacts = write_unit_artifacts(root, "unit-1", 192, 192)
            reference = root / "artifacts" / "unit-1-reference.json"
            value = json.loads(reference.read_text(encoding="utf-8"))
            value["unattributed_cells"] = [
                {
                    "worksheet": "Sheet1",
                    "address": "A1",
                    "number_format": "unsupported",
                    "reason": "number format display text is not reproduced",
                }
            ]
            reference.write_text(json.dumps(value, sort_keys=True), encoding="utf-8")
            # Rebind so the digest matches and the gate under test is reached
            # rather than the artifact hash check.
            artifacts["reference_inventory"] = binding(root, reference)

            with self.assertRaisesRegex(MetricError, "inventory.unattributed_cells"):
                _ = compute_unit(
                    {"artifacts": artifacts},
                    "unit-1",
                    frozenset({"content", "layout"}),
                    "#ffffff",
                    DocumentFormat.XLSX,
                    root,
                )

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
                        "unattributed_cells": [],
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
