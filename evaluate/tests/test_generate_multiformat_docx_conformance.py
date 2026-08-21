from __future__ import annotations

import json
import stat
import tempfile
import unittest
import xml.etree.ElementTree as ET
from collections import Counter
from pathlib import Path
from unittest.mock import patch
from zipfile import ZipFile

from evaluate.build_multiformat_conformance_plan import build_conformance_plan
from evaluate.generate_multiformat_docx_conformance import (
    DocxConformanceError,
    generate_docx_conformance,
)
from evaluate.multiformat_schema import sha256_file

PROJECT_ROOT = Path(__file__).resolve().parents[2]
CONTRACT = PROJECT_ROOT / "evaluate" / "multiformat" / "contract.v1.json"
WORD_NAMESPACE = "http://schemas.openxmlformats.org/wordprocessingml/2006/main"


class GenerateMultiFormatDocxConformanceTests(unittest.TestCase):
    def test_date_field_is_locked_to_frozen_value(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            plan = root / "plan.json"
            build_conformance_plan(CONTRACT, plan)
            manifest_path = generate_docx_conformance(CONTRACT, plan, root / "output")
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
            item = next(
                entry
                for entry in manifest["files"]
                if entry["primary_stratum"] == "lists-fields-references"
            )

            with ZipFile(manifest_path.parent / item["path"]) as archive:
                document = ET.fromstring(archive.read("word/document.xml"))

            field = document.find(
                ".//w:fldSimple",
                {"w": WORD_NAMESPACE},
            )
            self.assertIsNotNone(field)
            assert field is not None
            self.assertEqual(
                field.attrib.get(f"{{{WORD_NAMESPACE}}}instr"),
                ' DATE \\@ "yyyy-MM-dd" ',
            )
            self.assertEqual(
                field.attrib.get(f"{{{WORD_NAMESPACE}}}fldLock"),
                "true",
            )
            self.assertEqual(
                field.findtext(".//w:t", namespaces={"w": WORD_NAMESPACE}),
                "2000-01-01",
            )

    def test_materializes_exact_frozen_snapshot_contract(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            # Given
            root = Path(temp_dir)
            plan = root / "plan.json"
            build_conformance_plan(CONTRACT, plan)

            # When
            manifest_path = generate_docx_conformance(CONTRACT, plan, root / "output")

            # Then
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
            files = manifest["files"]
            self.assertEqual(len(files), 100)
            self.assertEqual(
                Counter(item["primary_stratum"] for item in files),
                Counter(
                    {
                        "text-typography": 25,
                        "sections-headers-footers": 20,
                        "tables-images-shapes": 20,
                        "lists-fields-references": 15,
                        "international": 10,
                        "mixed-stress": 10,
                    }
                ),
            )
            self.assertEqual(manifest["contract_sha256"], sha256_file(CONTRACT))
            self.assertEqual(manifest["plan_sha256"], sha256_file(plan))
            self.assertRegex(manifest["generator_sha256"], r"^[0-9a-f]{64}$")
            self.assertRegex(manifest["evaluator_sha256"], r"^[0-9a-f]{64}$")
            self.assertRegex(manifest["inventory_sha256"], r"^[0-9a-f]{64}$")
            self.assertEqual(
                manifest["snapshot_sha256"],
                "17d8c6658df47b72fdb81b14dcdb402526ba9d9c0ab02580300d727cf1d223ba",
            )
            self.assertEqual([item["ordinal"] for item in files], list(range(1, 101)))
            self.assertEqual(len({item["feature_seed"] for item in files}), 100)
            for item in files:
                source = manifest_path.parent / item["path"]
                self.assertEqual(sha256_file(source), item["sha256"])
                self.assertFalse(source.stat().st_mode & stat.S_IWUSR)
            self.assertEqual(
                len(
                    [path for path in manifest_path.parent.rglob("*") if path.is_file()]
                ),
                101,
            )

    def test_generation_is_reproducible_separately_from_frozen_identity(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            # Given
            root = Path(temp_dir)
            plan = root / "plan.json"
            build_conformance_plan(CONTRACT, plan)

            # When
            first = generate_docx_conformance(CONTRACT, plan, root / "first")
            second = generate_docx_conformance(CONTRACT, plan, root / "second")

            # Then
            self.assertEqual(first.read_bytes(), second.read_bytes())
            first_sources = sorted((first.parent / "sources" / "docx").iterdir())
            second_sources = sorted((second.parent / "sources" / "docx").iterdir())
            self.assertEqual(
                [path.read_bytes() for path in first_sources],
                [path.read_bytes() for path in second_sources],
            )

    def test_refuses_existing_output_without_modifying_it(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            # Given
            root = Path(temp_dir)
            plan = root / "plan.json"
            output = root / "output"
            output.mkdir()
            sentinel = output / "sentinel"
            sentinel.write_text("keep", encoding="utf-8")
            build_conformance_plan(CONTRACT, plan)

            # When / Then
            with self.assertRaises(DocxConformanceError):
                generate_docx_conformance(CONTRACT, plan, output)
            self.assertEqual(sentinel.read_text(encoding="utf-8"), "keep")

    def test_generation_failure_removes_partial_output(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            # Given
            root = Path(temp_dir)
            plan = root / "plan.json"
            output = root / "output"
            build_conformance_plan(CONTRACT, plan)

            # When
            with (
                patch(
                    "evaluate.generate_multiformat_docx_conformance.docx_case_bytes",
                    side_effect=OSError("injected failure"),
                ),
                self.assertRaises(DocxConformanceError),
            ):
                generate_docx_conformance(CONTRACT, plan, output)

            # Then
            self.assertFalse(output.exists())


if __name__ == "__main__":
    unittest.main()
