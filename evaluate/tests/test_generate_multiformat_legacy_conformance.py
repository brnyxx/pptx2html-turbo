from __future__ import annotations

import json
import shutil
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from evaluate.build_multiformat_conformance_plan import build_conformance_plan
from evaluate.multiformat_candidate_artifacts import write_canonical_json
from evaluate.multiformat_legacy_conformance import (
    LegacyConformanceError,
    LegacyPairGeneration,
    LegacyPairJob,
    LegacyPairRuntime,
    LegacyToolIdentity,
    generate_legacy_pairs,
)
from evaluate.multiformat_schema import sha256_file
from evaluate.tests.multiformat_source_fixture import write_positive_source

PROJECT_ROOT = Path(__file__).resolve().parents[2]
CONTRACT = PROJECT_ROOT / "evaluate" / "multiformat" / "contract.v1.json"
MODERN_FORMATS = ("docx", "xlsx", "pptx")


class GenerateMultiFormatLegacyConformanceTests(unittest.TestCase):
    def test_materializes_exact_sixty_pairs_for_every_legacy_format(self) -> None:
        # Given
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            plan, manifests = self._inputs(root)
            output = root / "legacy"

            # When
            manifest = generate_legacy_pairs(
                LegacyPairGeneration(CONTRACT, plan, manifests, output),
                LegacyPairRuntime(self._materialize, self._tools()),
            )

            # Then
            values = json.loads(manifest.read_text(encoding="utf-8"))
            self.assertEqual(values["status"], "GENERATED")
            self.assertEqual(set(values["formats"]), {"doc", "xls", "ppt"})
            for legacy_format, paired_format in (
                ("doc", "docx"),
                ("xls", "xlsx"),
                ("ppt", "pptx"),
            ):
                with self.subTest(document_format=legacy_format):
                    files = values["formats"][legacy_format]["files"]
                    self.assertEqual(len(files), 60)
                    self.assertEqual(
                        [item["ordinal"] for item in files],
                        list(range(1, 61)),
                    )
                    self.assertTrue(
                        all(
                            item["primary_stratum"] == "paired-legacy" for item in files
                        )
                    )
                    self.assertTrue(
                        all(
                            item["paired_source"]["id"].startswith(paired_format)
                            for item in files
                        )
                    )
                    for item in files:
                        source = output / item["path"]
                        paired = output / item["paired_source"]["path"]
                        self.assertEqual(sha256_file(source), item["sha256"])
                        self.assertEqual(
                            sha256_file(paired),
                            item["paired_source"]["sha256"],
                        )
            self.assertEqual(
                len([path for path in output.rglob("*") if path.is_file()]),
                361,
            )

    def test_modern_source_drift_fails_before_conversion(self) -> None:
        # Given
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            plan, manifests = self._inputs(root)
            values = json.loads(manifests[0].read_text(encoding="utf-8"))
            source = manifests[0].parent / values["files"][0]["path"]
            source.write_bytes(b"changed")
            calls = 0

            def materialize(job: LegacyPairJob) -> int:
                nonlocal calls
                calls += 1
                return self._materialize(job)

            # When / Then
            with self.assertRaisesRegex(
                LegacyConformanceError,
                "modern source binding differs",
            ):
                generate_legacy_pairs(
                    LegacyPairGeneration(
                        CONTRACT,
                        plan,
                        manifests,
                        root / "legacy",
                    ),
                    LegacyPairRuntime(materialize, self._tools()),
                )

            self.assertEqual(calls, 0)
            self.assertFalse((root / "legacy").exists())

    def test_unit_count_drift_removes_partial_snapshot(self) -> None:
        # Given
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            plan, manifests = self._inputs(root)
            calls = 0

            def wrong_count(job: LegacyPairJob) -> int:
                nonlocal calls
                calls += 1
                self._materialize(job)
                return 2 if calls == 2 else 1

            # When / Then
            with self.assertRaisesRegex(
                LegacyConformanceError,
                "one visual unit",
            ):
                generate_legacy_pairs(
                    LegacyPairGeneration(
                        CONTRACT,
                        plan,
                        manifests,
                        root / "legacy",
                    ),
                    LegacyPairRuntime(wrong_count, self._tools()),
                )

            self.assertFalse((root / "legacy").exists())

    def test_existing_output_is_never_modified(self) -> None:
        # Given
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            plan, manifests = self._inputs(root)
            output = root / "legacy"
            output.mkdir()
            marker = output / "owner.txt"
            marker.write_text("keep", encoding="utf-8")

            # When / Then
            with self.assertRaisesRegex(
                LegacyConformanceError,
                "output already exists",
            ):
                generate_legacy_pairs(
                    LegacyPairGeneration(CONTRACT, plan, manifests, output),
                    LegacyPairRuntime(self._materialize, self._tools()),
                )

            self.assertEqual(marker.read_text(encoding="utf-8"), "keep")

    def test_immutability_failure_never_publishes_snapshot(self) -> None:
        # Given
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            plan, manifests = self._inputs(root)
            output = root / "legacy"

            # When / Then
            with (
                mock.patch.object(
                    Path,
                    "chmod",
                    side_effect=OSError("forced chmod failure"),
                ),
                self.assertRaises(LegacyConformanceError),
            ):
                generate_legacy_pairs(
                    LegacyPairGeneration(CONTRACT, plan, manifests, output),
                    LegacyPairRuntime(self._materialize, self._tools()),
                )

            self.assertFalse(output.exists())

    def test_distinct_modern_sources_with_same_basename_remain_distinct(self) -> None:
        # Given
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            plan, manifests = self._inputs(root)
            manifest = manifests[0]
            values = json.loads(manifest.read_text(encoding="utf-8"))
            for index, directory in enumerate(("first", "second")):
                item = values["files"][index]
                original = manifest.parent / item["path"]
                collision = manifest.parent / "collision" / directory / "shared.docx"
                collision.parent.mkdir(parents=True)
                shutil.copyfile(original, collision)
                item["path"] = collision.relative_to(manifest.parent).as_posix()
                item["sha256"] = sha256_file(collision)
            write_canonical_json(manifest, values)
            output = root / "legacy"

            # When
            generated = generate_legacy_pairs(
                LegacyPairGeneration(CONTRACT, plan, manifests, output),
                LegacyPairRuntime(self._materialize, self._tools()),
            )

            # Then
            result = json.loads(generated.read_text(encoding="utf-8"))
            first, second = result["formats"]["doc"]["files"][:2]
            self.assertNotEqual(
                first["paired_source"]["path"],
                second["paired_source"]["path"],
            )
            for item in (first, second):
                paired = item["paired_source"]
                self.assertEqual(
                    sha256_file(output / paired["path"]),
                    paired["sha256"],
                )

    def _inputs(self, root: Path) -> tuple[Path, tuple[Path, ...]]:
        plan = root / "plan.json"
        build_conformance_plan(CONTRACT, plan)
        plan_values = json.loads(plan.read_text(encoding="utf-8"))
        manifests: list[Path] = []
        for document_format in MODERN_FORMATS:
            snapshot = root / document_format
            source_root = snapshot / "sources" / document_format
            source_root.mkdir(parents=True)
            files = []
            for case in plan_values["formats"][document_format]["cases"]:
                source = source_root / f"{case['id']}.{document_format}"
                write_positive_source(source, document_format, case["id"])
                files.append(
                    {
                        "id": case["id"],
                        "ordinal": case["ordinal"],
                        "primary_stratum": case["primary_stratum"],
                        "path": source.relative_to(snapshot).as_posix(),
                        "sha256": sha256_file(source),
                        "unit_count": 1,
                    }
                )
            manifest = snapshot / "generation-manifest.json"
            write_canonical_json(
                manifest,
                {
                    "schema_version": 1,
                    "status": "GENERATED",
                    "format": document_format,
                    "contract_sha256": sha256_file(CONTRACT),
                    "plan_sha256": sha256_file(plan),
                    "files": files,
                },
            )
            manifests.append(manifest)
        return plan, tuple(manifests)

    def _materialize(self, job: LegacyPairJob) -> int:
        write_positive_source(
            job.destination,
            job.document_format.value,
            job.case_id,
        )
        return 1

    def _tools(self) -> LegacyToolIdentity:
        return LegacyToolIdentity(
            soffice_sha256="1" * 64,
            soffice_version="LibreOffice test",
            pdfinfo_sha256="2" * 64,
            pdfinfo_version="pdfinfo test",
            font_environment_sha256="3" * 64,
        )


if __name__ == "__main__":
    unittest.main()
