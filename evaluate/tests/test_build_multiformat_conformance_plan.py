from __future__ import annotations

import json
import tempfile
import unittest
from collections import Counter
from pathlib import Path

from evaluate.build_multiformat_conformance_plan import build_conformance_plan

PROJECT_ROOT = Path(__file__).resolve().parents[2]
CONTRACT = PROJECT_ROOT / "evaluate" / "multiformat" / "contract.v1.json"


class BuildMultiFormatConformancePlanTests(unittest.TestCase):
    def test_plan_expands_exact_contract_quotas_and_legacy_pairs(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            output = Path(temp_dir) / "plan.json"

            build_conformance_plan(CONTRACT, output)

            contract = json.loads(CONTRACT.read_text(encoding="utf-8"))
            plan = json.loads(output.read_text(encoding="utf-8"))
            self.assertEqual(set(plan["formats"]), set(contract["required_formats"]))
            for document_format, values in plan["formats"].items():
                cases = values["cases"]
                self.assertEqual(len(cases), 100)
                self.assertEqual(
                    Counter(item["primary_stratum"] for item in cases),
                    Counter(contract["stratum_quotas"][document_format]),
                )
                self.assertEqual(
                    len({item["id"] for item in cases}),
                    100,
                )
            for document_format in ["doc", "xls", "ppt"]:
                cases = plan["formats"][document_format]["cases"]
                paired = [item for item in cases if item["paired_case_id"] is not None]
                binary = [item for item in cases if item["paired_case_id"] is None]
                self.assertEqual(len(paired), 60)
                self.assertEqual(len(binary), 40)
                self.assertEqual(
                    Counter(item["paired_stratum"] for item in paired),
                    Counter(contract["legacy_paired_stratum_quotas"][document_format]),
                )

    def test_plan_is_byte_deterministic(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            first = root / "first.json"
            second = root / "second.json"

            build_conformance_plan(CONTRACT, first)
            build_conformance_plan(CONTRACT, second)

            self.assertEqual(first.read_bytes(), second.read_bytes())
