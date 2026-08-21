from __future__ import annotations

import io
import json
import stat
import tempfile
import unittest
import zipfile
from collections import Counter
from pathlib import Path
from unittest import mock

from evaluate.build_multiformat_conformance_plan import build_conformance_plan
from evaluate.generate_multiformat_pptx_conformance import generate_pptx_conformance
from evaluate.multiformat_conformance_pptx import (
    PptxConformanceError,
    admit_pptx_case,
    pptx_case_bytes,
)
from evaluate.multiformat_schema import sha256_file

PROJECT_ROOT = Path(__file__).resolve().parents[2]
CONTRACT = PROJECT_ROOT / "evaluate" / "multiformat" / "contract.v1.json"
QUOTAS = {
    "text": 20,
    "shapes-connectors": 20,
    "images-effects": 15,
    "tables-charts": 15,
    "masters-layouts-groups": 15,
    "international": 10,
    "fallback-edge": 5,
}
FEATURE_TOKENS = {
    "text": (b'<a:rPr b="1" i="1"',),
    "shapes-connectors": (b'prst="roundRect"', b"<p:cxnSp>"),
    "images-effects": (b"<p:pic>", b"<a:effectLst>"),
    "tables-charts": (b"<a:tbl>", b"<c:chart "),
    "masters-layouts-groups": (b"<p:grpSp>",),
    "international": ("한글 العربية 日本語".encode(),),
    "fallback-edge": (b"<mc:AlternateContent>", b"<mc:Fallback>"),
}


class GenerateMultiFormatPptxConformanceTests(unittest.TestCase):
    def test_generates_exact_immutable_snapshot_with_complete_bindings(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            plan = root / "plan.json"
            build_conformance_plan(CONTRACT, plan)

            manifest_path = generate_pptx_conformance(CONTRACT, plan, root / "output")

            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
            files = manifest["files"]
            self.assertEqual(len(files), 100)
            self.assertEqual(Counter(item["primary_stratum"] for item in files), QUOTAS)
            self.assertEqual(
                [item["id"] for item in files],
                [f"pptx-conformance-{ordinal:03d}" for ordinal in range(1, 101)],
            )
            self.assertEqual(manifest["contract_sha256"], sha256_file(CONTRACT))
            self.assertEqual(manifest["plan_sha256"], sha256_file(plan))
            self.assertEqual(manifest["snapshot_identity"]["kind"], "frozen-sources")
            self.assertEqual(
                manifest["generator_reproducibility"]["kind"],
                "deterministic-generator",
            )
            self.assertNotEqual(
                manifest["snapshot_identity"]["sha256"],
                manifest["generator_reproducibility"]["sha256"],
            )
            for item in files:
                source = manifest_path.parent / item["path"]
                self.assertEqual(item["sha256"], sha256_file(source))
                self.assertEqual(stat.S_IMODE(source.stat().st_mode), 0o444)
                self.assertGreater(len(item["package_inventory"]), 10)
                with zipfile.ZipFile(source) as archive:
                    slide = archive.read("ppt/slides/slide1.xml")
                    self.assertEqual(
                        [entry["path"] for entry in item["package_inventory"]],
                        archive.namelist(),
                    )
                    self.assertTrue(
                        all(
                            token in slide
                            for token in FEATURE_TOKENS[item["primary_stratum"]]
                        ),
                        item["id"],
                    )
                    self.assertIn(
                        b'<a:off x="500000" y="5400000"/>'
                        b'<a:ext cx="8200000" cy="1200000"/>',
                        slide,
                    )
                    self.assertTrue(
                        all(
                            info.date_time == (1980, 1, 1, 0, 0, 0)
                            for info in archive.infolist()
                        )
                    )
            self.assertEqual(
                len(
                    [path for path in manifest_path.parent.rglob("*") if path.is_file()]
                ),
                101,
            )

    def test_clean_runs_are_byte_identical(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            plan = root / "plan.json"
            build_conformance_plan(CONTRACT, plan)

            first = generate_pptx_conformance(CONTRACT, plan, root / "first")
            second = generate_pptx_conformance(CONTRACT, plan, root / "second")

            self.assertEqual(
                json.loads(first.read_text(encoding="utf-8")),
                json.loads(second.read_text(encoding="utf-8")),
            )
            self.assertEqual(
                [
                    sha256_file(path)
                    for path in sorted((root / "first/sources/pptx").iterdir())
                ],
                [
                    sha256_file(path)
                    for path in sorted((root / "second/sources/pptx").iterdir())
                ],
            )

    def test_admission_rejects_missing_required_stratum_features(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            plan = Path(temp_dir) / "plan.json"
            build_conformance_plan(CONTRACT, plan)
            cases = json.loads(plan.read_text(encoding="utf-8"))["formats"]["pptx"][
                "cases"
            ]
            by_stratum = {case["primary_stratum"]: case for case in cases}
            mutations = {
                "shapes-connectors": (b'prst="roundRect"', b'prst="ellipse"'),
                "images-effects": (b"effectLst", b"effectLsx"),
                "tables-charts": (b"c:chart", b"c:chort"),
                "fallback-edge": (b"mc:Fallback", b"mc:Fallbacc"),
            }

            for stratum, (old, new) in mutations.items():
                with self.subTest(stratum=stratum):
                    case = by_stratum[stratum]
                    mutated = _replace_slide_xml(pptx_case_bytes(case), old, new)
                    with self.assertRaises(PptxConformanceError):
                        admit_pptx_case(mutated, case)

    def test_refuses_preexisting_output_without_modifying_it(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            plan = root / "plan.json"
            output = root / "output"
            output.mkdir()
            sentinel = output / "keep.txt"
            sentinel.write_text("keep", encoding="utf-8")
            build_conformance_plan(CONTRACT, plan)

            with self.assertRaisesRegex(PptxConformanceError, "already exists"):
                generate_pptx_conformance(CONTRACT, plan, output)

            self.assertEqual(sentinel.read_text(encoding="utf-8"), "keep")

    def test_removes_partial_output_when_materialization_fails(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            plan = root / "plan.json"
            output = root / "output"
            build_conformance_plan(CONTRACT, plan)

            with (
                mock.patch(
                    "evaluate.generate_multiformat_pptx_conformance.pptx_case_bytes",
                    side_effect=OSError("injected failure"),
                ),
                self.assertRaisesRegex(PptxConformanceError, "generation failed"),
            ):
                generate_pptx_conformance(CONTRACT, plan, output)

            self.assertFalse(output.exists())


def _replace_slide_xml(value: bytes, old: bytes, new: bytes) -> bytes:
    source = zipfile.ZipFile(io.BytesIO(value))
    output = io.BytesIO()
    with source, zipfile.ZipFile(output, "w") as target:
        for information in source.infolist():
            data = source.read(information)
            if information.filename == "ppt/slides/slide1.xml":
                data = data.replace(old, new)
            target.writestr(information, data)
    return output.getvalue()


if __name__ == "__main__":
    unittest.main()
