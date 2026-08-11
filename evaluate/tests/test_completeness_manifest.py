from __future__ import annotations

import contextlib
import io
import json
import tempfile
import unittest
from pathlib import Path
from evaluate.check_completeness_manifest import (
    CONTRACT_SCOPE,
    REQUIRED_FEATURE_IDS,
    load_manifest,
    main,
    validate_manifest,
)


def manifest_path_for_repository() -> Path:
    return Path(__file__).resolve().parents[2] / "evaluate/completeness_manifest.json"


def contract_path_for_repository() -> Path:
    return (
        Path(__file__).resolve().parents[2]
        / "docs/architecture/PPTX_COMPLETENESS_CONTRACT.md"
    )


class CompletenessManifestTests(unittest.TestCase):
    def test_rejects_unclassified_dimension_tier(self) -> None:
        manifest = _valid_manifest()
        manifest["features"][0]["current"]["visual"]["tier"] = "direct"

        errors = validate_manifest(manifest)

        self.assertIn("UNCLASSIFIED_TIER:presentation:visual", errors)

    def test_rejects_missing_official_source(self) -> None:
        manifest = _valid_manifest()
        del manifest["features"][0]["official_source"]

        errors = validate_manifest(manifest)

        self.assertIn("MISSING_OFFICIAL_SOURCE:presentation", errors)

    def test_rejects_exact_dimension_without_powerpoint_evidence(self) -> None:
        manifest = _valid_manifest()
        manifest["features"][0]["current"]["visual"]["tier"] = "exact"

        errors = validate_manifest(manifest)

        self.assertIn("EXACT_REQUIRES_POWERPOINT_EVIDENCE:presentation:visual", errors)

    def test_rejects_non_object_manifest_root_without_traceback(self) -> None:
        errors = validate_manifest(["not", "a", "manifest"])

        self.assertEqual(errors, ["MANIFEST_ROOT_MUST_BE_OBJECT"])

    def test_cli_rejects_malformed_json_with_stable_code(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            path = Path(tmpdir) / "manifest.json"
            path.write_text("{", encoding="utf-8")

            exit_code, output = self._run_cli(path)

        self.assertEqual(exit_code, 1)
        self.assertEqual(output, "MANIFEST_JSON_INVALID\n")

    def test_cli_rejects_invalid_unicode_with_stable_code(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            path = Path(tmpdir) / "manifest.json"
            path.write_bytes(b"\xff")

            exit_code, output = self._run_cli(path)

        self.assertEqual(exit_code, 1)
        self.assertEqual(output, "MANIFEST_TEXT_INVALID\n")

    def test_cli_rejects_unreadable_manifest_path_with_stable_code(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            exit_code, output = self._run_cli(Path(tmpdir))

        self.assertEqual(exit_code, 1)
        self.assertEqual(output, "MANIFEST_READ_FAILED\n")

    def test_cli_rejects_non_object_json_root_with_stable_code(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            path = Path(tmpdir) / "manifest.json"
            path.write_text("[]", encoding="utf-8")

            exit_code, output = self._run_cli(path)

        self.assertEqual(exit_code, 1)
        self.assertEqual(output, "MANIFEST_ROOT_MUST_BE_OBJECT\n")

    def test_rejects_unofficial_source_urls(self) -> None:
        sources = (
            "https://example.invalid/official",
            "https://user@learn.microsoft.com/path",
            "https://learn.microsoft.com:8443/path",
            "https://learn.microsoft.com",
        )
        for source in sources:
            with self.subTest(source=source):
                manifest = _valid_manifest()
                manifest["features"][0]["official_source"] = source

                errors = validate_manifest(manifest)

                self.assertIn("UNOFFICIAL_SOURCE:presentation", errors)

    def test_rejects_blank_fallback_policy_values(self) -> None:
        manifest = _valid_manifest()
        manifest["features"][0]["fallback_policy"]["kind"] = "  "
        manifest["features"][0]["fallback_policy"]["diagnostic_code"] = ""

        errors = validate_manifest(manifest)

        self.assertIn("INVALID_FALLBACK_POLICY:presentation", errors)

    def test_rejects_mutated_canonical_row_metadata(self) -> None:
        mutations = (
            ("official_source", "https://learn.microsoft.com/en-us/dotnet/api/"),
            ("source_status", "unavailable"),
            ("ooxml", {"qualified_name": "p:sld"}),
            (
                "fallback_policy",
                {"kind": "arbitrary", "diagnostic_code": "ARBITRARY"},
            ),
        )
        for field, value in mutations:
            with self.subTest(field=field):
                manifest = _valid_manifest()
                manifest["features"][0][field] = value

                errors = validate_manifest(manifest)

                self.assertIn("CANONICAL_ROW_MISMATCH:presentation", errors)

    def test_rejects_exact_evidence_when_existing_files_fail_the_gate(self) -> None:
        manifest = _valid_manifest()
        feature = manifest["features"][0]
        feature["current"]["visual"]["tier"] = "exact"
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            (root / "capture.json").write_text("{}", encoding="utf-8")
            (root / "fixtures").mkdir()
            (root / "artifact.png").write_bytes(b"artifact")
            (root / "golden").mkdir()
            (root / "output").mkdir()
            feature["exact_evidence"] = {
                "oracle": "PowerPoint-native",
                "powerpoint_version": "test",
                "windows_version": "test",
                "capture_metadata": "capture.json",
                "fixture_bundle": "fixtures",
                "artifact_paths": ["artifact.png"],
                "gate_family": "text-layout",
                "golden_set_dir": "golden",
                "output_dir": "output",
            }

            errors = validate_manifest(manifest, root)

        self.assertIn("EXACT_REQUIRES_POWERPOINT_EVIDENCE:presentation:visual", errors)

    def test_rejects_invalid_root_contract_metadata(self) -> None:
        manifest = _valid_manifest()
        manifest["schema_version"] = "3.0"
        manifest["exact_promotion_gate"] = {
            "code": "wrong",
            "oracle": "browser",
            "evidence_status": "optional",
        }

        errors = validate_manifest(manifest)

        self.assertIn("INVALID_SCHEMA_VERSION", errors)
        self.assertIn("INVALID_EXACT_PROMOTION_GATE", errors)

    def test_rejects_modified_contract_scope(self) -> None:
        manifest = _valid_manifest()
        manifest["contract_scope"] = "broader than the frozen contract"

        errors = validate_manifest(manifest)

        self.assertIn("INVALID_CONTRACT_SCOPE", errors)

    def test_rejects_feature_id_outside_lowercase_kebab_case(self) -> None:
        manifest = _valid_manifest()
        manifest["features"][0]["id"] = "Presentation_Master"

        errors = validate_manifest(manifest)

        self.assertIn("INVALID_FEATURE_ID:Presentation_Master", errors)

    def test_rejects_missing_current_disposition_dimension(self) -> None:
        manifest = _valid_manifest()
        manifest["features"][0]["current"] = {
            "semantic": {"tier": "approximate", "stage": "parsed"}
        }

        errors = validate_manifest(manifest)

        self.assertIn("MISSING_CURRENT_DIMENSION:presentation:visual", errors)

    def test_rejects_missing_target_disposition_dimension(self) -> None:
        manifest = _valid_manifest()
        del manifest["features"][0]["target"]["visual"]

        errors = validate_manifest(manifest)

        self.assertIn("MISSING_TARGET_DIMENSION:presentation:visual", errors)

    def test_rejects_extra_current_disposition_dimension(self) -> None:
        manifest = _valid_manifest()
        manifest["features"][0]["current"]["auxiliary"] = {
            "tier": "fallback",
            "stage": "not-applicable",
        }

        errors = validate_manifest(manifest)

        self.assertIn("INVALID_CURRENT_DIMENSIONS:presentation", errors)

    def test_rejects_unexpected_feature_id_from_the_frozen_inventory(self) -> None:
        manifest = _valid_manifest()
        unexpected_feature = json.loads(json.dumps(manifest["features"][0]))
        unexpected_feature["id"] = "unexpected-relationship"
        manifest["features"].append(unexpected_feature)

        errors = validate_manifest(manifest)

        self.assertIn("UNEXPECTED_FEATURE_ID:unexpected-relationship", errors)

    def test_rejects_missing_new_official_inventory_row(self) -> None:
        manifest = _valid_manifest()
        manifest["features"] = [
            feature
            for feature in manifest["features"]
            if feature["id"] != "content-part"
        ]

        errors = validate_manifest(manifest)

        self.assertIn("MISSING_REQUIRED_FEATURE:content-part", errors)

    def test_rejects_deletion_of_each_frozen_inventory_row(self) -> None:
        for feature_id in REQUIRED_FEATURE_IDS:
            with self.subTest(feature_id=feature_id):
                manifest = _valid_manifest()
                manifest["features"] = [
                    feature
                    for feature in manifest["features"]
                    if feature["id"] != feature_id
                ]

                errors = validate_manifest(manifest)

                self.assertIn(f"MISSING_REQUIRED_FEATURE:{feature_id}", errors)

    def test_rejects_qualified_name_outside_the_official_inventory(self) -> None:
        manifest = _valid_manifest()
        manifest["features"][0]["ooxml"] = {"qualified_name": "p:notOfficial"}

        errors = validate_manifest(manifest)

        self.assertIn("UNKNOWN_QUALIFIED_NAME:presentation", errors)

    def test_repository_inventory_contains_separated_subfeatures_and_parts(
        self,
    ) -> None:
        manifest = _valid_manifest()
        feature_ids = {feature["id"] for feature in manifest["features"]}

        self.assertTrue(
            {
                "bullets",
                "picture-bullets",
                "fills",
                "pattern-fill",
                "effects",
                "reflection-and-3d",
                "chart-direct-subset",
                "chart-preview-fallback",
                "chart-placeholder-fallback",
                "hyperlink-run-and-cell",
                "shape-hyperlink-and-action",
                "additional-characteristics",
                "custom-xml",
                "thumbnail",
                "theme-override",
                "slide-synchronization",
                "content-part",
                "embedded-package",
                "user-defined-tags",
                "bibliography",
                "embedded-control-persistence",
                "table-style",
                "rtl-text",
            }.issubset(feature_ids)
        )

    def test_rejects_blank_family_and_invalid_exact_evidence_shape(self) -> None:
        manifest = _valid_manifest()
        feature = manifest["features"][0]
        feature["family"] = " "
        feature["current"]["visual"]["tier"] = "exact"
        feature["exact_evidence"] = {
            "oracle": "PowerPoint-native",
            "powerpoint_version": [],
            "windows_version": "Windows",
            "capture_metadata": "metadata.json",
            "fixture_bundle": "fixtures",
            "artifact_paths": [""],
        }

        errors = validate_manifest(manifest)

        self.assertIn("MISSING_FEATURE_FAMILY:presentation", errors)
        self.assertIn("EXACT_REQUIRES_POWERPOINT_EVIDENCE:presentation:visual", errors)

    def test_repository_manifest_is_a_valid_contract(self) -> None:
        manifest = load_manifest(manifest_path_for_repository())

        self.assertEqual(validate_manifest(manifest), [])

    def test_contract_document_exposes_stable_contract_boundaries(self) -> None:
        contract = contract_path_for_repository().read_text(encoding="utf-8")

        self.assertIn("## Purpose", contract)
        self.assertIn("## Exact-promotion gate", contract)
        self.assertIn("evaluate/completeness_manifest.json", contract)
        self.assertIn(
            f"`contract_scope` must exactly equal `{CONTRACT_SCOPE}`", contract
        )
        self.assertIn("EXACT_REQUIRES_POWERPOINT_EVIDENCE", contract)

    def _run_cli(self, manifest_path: Path) -> tuple[int, str]:
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            exit_code = main(["--manifest", str(manifest_path)])
        return exit_code, output.getvalue()


def _valid_manifest() -> dict[str, object]:
    manifest = load_manifest(manifest_path_for_repository())
    if not isinstance(manifest, dict):
        raise AssertionError("Repository manifest must be an object")
    return json.loads(json.dumps(manifest))
