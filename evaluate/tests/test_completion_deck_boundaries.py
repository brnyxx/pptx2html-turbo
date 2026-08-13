from __future__ import annotations

import tempfile
import unittest
from dataclasses import replace
from enum import StrEnum
from pathlib import Path
from unittest import mock
from xml.etree import ElementTree

from evaluate import create_completion_decks
from evaluate.completion_deck_common import theme_xml
from evaluate.completion_deck_features import FEATURES, FeatureSpec, SchemaExpectation
from evaluate.completion_deck_manifest import ContractError
from evaluate.completion_deck_package import Deck, _content_types, relationships_xml
from evaluate.tests.completion_deck_test_support import CANONICAL_MANIFEST


class CompletionDeckBoundaryTests(unittest.TestCase):
    def _feature(self, feature_id: str) -> tuple[int, FeatureSpec]:
        return next(
            (index, feature)
            for index, feature in enumerate(FEATURES)
            if feature.feature_id == feature_id
        )

    def _replace_feature(
        self, feature_id: str, **changes: object
    ) -> tuple[FeatureSpec, ...]:
        index, feature = self._feature(feature_id)
        return (
            *FEATURES[:index],
            replace(feature, **changes),
            *FEATURES[index + 1 :],
        )

    def _assert_schema_rejected(
        self, features: tuple[FeatureSpec, ...], expected_code: str
    ) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / "out"
            with mock.patch.object(create_completion_decks, "FEATURES", features):
                with self.assertRaisesRegex(ContractError, expected_code):
                    create_completion_decks.generate(output, CANONICAL_MANIFEST)
            self.assertFalse(output.exists())

    def test_unexpected_negative_schema_is_rejected_before_write(self) -> None:
        features = self._replace_feature(
            "adjustment-basic",
            schema_expectation=SchemaExpectation.NEGATIVE,
            expected_diagnostic="PPTX_COMPLETENESS_FALLBACK",
        )
        self._assert_schema_rejected(
            features, "COMPLETION_SCHEMA_CONTRACT.*adjustment-basic"
        )

    def test_negative_schema_requires_exact_diagnostic(self) -> None:
        for diagnostic in (None, "WRONG"):
            with self.subTest(diagnostic=diagnostic):
                features = self._replace_feature(
                    "pattern-fill-unknown", expected_diagnostic=diagnostic
                )
                self._assert_schema_rejected(
                    features,
                    "COMPLETION_SCHEMA_CONTRACT.*pattern-fill-unknown",
                )

    def test_positive_schema_rejects_diagnostic(self) -> None:
        features = self._replace_feature(
            "adjustment-basic", expected_diagnostic="UNEXPECTED"
        )
        self._assert_schema_rejected(
            features, "COMPLETION_SCHEMA_CONTRACT.*adjustment-basic"
        )

    def test_invalid_schema_enum_is_rejected(self) -> None:
        features = self._replace_feature("adjustment-basic", schema_expectation="invalid")
        self._assert_schema_rejected(
            features,
            "COMPLETION_SCHEMA_EXPECTATION_INVALID.*adjustment-basic",
        )

    def test_schema_expectation_is_a_closed_enum(self) -> None:
        self.assertTrue(
            all(isinstance(feature.schema_expectation, StrEnum) for feature in FEATURES)
        )
        negative = [
            (
                feature.feature_id,
                feature.schema_expectation,
                feature.expected_diagnostic,
            )
            for feature in FEATURES
            if feature.schema_expectation is SchemaExpectation.NEGATIVE
        ]
        self.assertEqual(
            negative,
            [
                (
                    "pattern-fill-unknown",
                    SchemaExpectation.NEGATIVE,
                    "DRAWINGML_PATTERN_UNSUPPORTED",
                )
            ],
        )
        positive = [
            feature
            for feature in FEATURES
            if feature.schema_expectation is SchemaExpectation.POSITIVE
        ]
        self.assertEqual(len(positive), len(FEATURES) - 1)
        self.assertTrue(
            all(feature.expected_diagnostic is None for feature in positive)
        )

    def test_dynamic_xml_attributes_round_trip_hostile_text(self) -> None:
        hostile = 'bad"&<>'
        theme = ElementTree.fromstring(theme_xml(hostile))
        self.assertEqual(theme.get("name"), hostile)

        relationships = ElementTree.fromstring(
            relationships_xml(((hostile, hostile, hostile, hostile),))
        )
        relation = relationships[0]
        self.assertEqual(
            tuple(
                relation.get(name) for name in ("Id", "Type", "Target", "TargetMode")
            ),
            (hostile, hostile, hostile, hostile),
        )

        deck = Deck("hostile", (("", ""),), types=((hostile, hostile),))
        content_types = ElementTree.fromstring(_content_types(deck))
        override = content_types[-2]
        self.assertEqual(
            (override.get("PartName"), override.get("ContentType")),
            (hostile, hostile),
        )


if __name__ == "__main__":
    unittest.main()
