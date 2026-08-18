import importlib.util
import json
import unittest
from collections import Counter
from pathlib import Path

from evaluate import adjustment_cases

ROOT = Path(__file__).resolve().parents[2]
MANIFEST = ROOT / "evaluate" / "preset_adjustments.json"


class AdjustmentCasesTests(unittest.TestCase):
    def test_adjustment_case_module_exists(self) -> None:
        module = importlib.util.find_spec("evaluate.adjustment_cases")

        self.assertIsNotNone(module)

    def test_inventory_contains_every_official_adjustment_pair(self) -> None:
        # Given
        payload = json.loads(MANIFEST.read_text(encoding="utf-8"))
        expected = [
            (preset["name"], adjustment["name"])
            for preset in payload["presets"]
            for adjustment in preset["adjustments"]
        ]

        # When
        specs = adjustment_cases.load_adjustment_specs(MANIFEST)

        # Then
        self.assertEqual(payload["contract"]["preset_count"], 187)
        self.assertEqual(len(expected), 300)
        self.assertEqual(
            [(spec.preset, spec.key) for spec in specs],
            expected,
        )

    def test_every_pair_has_low_default_and_high_cases(self) -> None:
        # Given
        specs = adjustment_cases.load_adjustment_specs(MANIFEST)

        # When
        cases = adjustment_cases.build_adjustment_cases(specs)

        # Then
        self.assertEqual(len(cases), 900)
        variants = Counter(
            (case.preset, case.key, case.variant) for case in cases
        )
        self.assertEqual(set(variants.values()), {1})
        for spec in specs:
            self.assertEqual(
                {
                    case.variant
                    for case in cases
                    if (case.preset, case.key) == (spec.preset, spec.key)
                },
                {"low", "default", "high"},
            )

    def test_each_case_changes_only_its_target_adjustment(self) -> None:
        # Given
        specs = adjustment_cases.load_adjustment_specs(MANIFEST)
        defaults_by_preset = {
            spec.preset: spec.defaults
            for spec in specs
        }

        # When
        cases = adjustment_cases.build_adjustment_cases(specs)

        # Then
        for case in cases:
            expected = dict(defaults_by_preset[case.preset])
            expected[case.key] = case.value
            self.assertEqual(case.adjustments, expected)

    def test_variant_values_are_distinct_and_deterministic(self) -> None:
        # Given
        specs = adjustment_cases.load_adjustment_specs(MANIFEST)

        # When
        first = adjustment_cases.build_adjustment_cases(specs)
        second = adjustment_cases.build_adjustment_cases(specs)

        # Then
        self.assertEqual(first, second)
        values: dict[tuple[str, str], set[int]] = {}
        for case in first:
            values.setdefault((case.preset, case.key), set()).add(case.value)
        self.assertTrue(all(len(pair_values) == 3 for pair_values in values.values()))

    def test_probe_values_avoid_extreme_and_unproven_ranges(self) -> None:
        specs = adjustment_cases.load_adjustment_specs(MANIFEST)
        cases = adjustment_cases.build_adjustment_cases(specs)
        by_pair = {
            (spec.preset, spec.key): [
                case
                for case in cases
                if (case.preset, case.key) == (spec.preset, spec.key)
                and case.variant != "default"
            ]
            for spec in specs
        }

        for spec in specs:
            probes = by_pair[(spec.preset, spec.key)]
            maximum_delta = (
                5_400_000 if abs(spec.default) >= 1_000_000 else 50_000
            )
            self.assertTrue(
                all(
                    abs(case.value - spec.default) <= maximum_delta
                    for case in probes
                )
            )
            if spec.lower is not None and spec.upper is None:
                self.assertTrue(
                    all(spec.lower <= case.value <= spec.default for case in probes)
                )
            if spec.lower is None and spec.upper is not None:
                self.assertTrue(
                    all(spec.default <= case.value <= spec.upper for case in probes)
                )

    def test_every_case_classifies_range_verification(self) -> None:
        specs = adjustment_cases.load_adjustment_specs(MANIFEST)
        cases = adjustment_cases.build_adjustment_cases(specs)

        self.assertEqual(
            {
                case.range_verification
                for case in cases
            },
            {
                "numeric-bounds",
                "default-interpolation",
                "symbolic-unverified",
                "range-unavailable",
            },
        )


if __name__ == "__main__":
    unittest.main()
