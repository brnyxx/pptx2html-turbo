from __future__ import annotations

import argparse
import contextlib
import io
import json
import tempfile
import unittest
from collections.abc import Mapping, Sequence
from pathlib import Path
from urllib.parse import urlsplit


DIMENSIONS = ("semantic", "visual", "behavioral")
TIERS = ("exact", "approximate", "fallback", "unparsed")
STAGES = ("parsed", "resolved", "rendered", "fidelity-tested", "not-applicable")
REQUIRED_FALLBACK_METADATA = (
    "code",
    "family",
    "tier",
    "stage",
    "slide_index",
    "part_name",
    "relationship_id",
    "relationship_type",
    "qualified_name",
    "bounds",
    "raw_reference",
    "fallback_kind",
    "reason",
)
REQUIRED_EXACT_EVIDENCE = (
    "oracle",
    "powerpoint_version",
    "windows_version",
    "capture_metadata",
    "fixture_bundle",
    "artifact_paths",
)
APPROVED_OFFICIAL_SOURCE_HOSTS = frozenset(
    {"learn.microsoft.com", "ecma-international.org"}
)
EXACT_PROMOTION_GATE = {
    "code": "EXACT_REQUIRES_POWERPOINT_EVIDENCE",
    "oracle": "PowerPoint-native",
    "evidence_status": "required-before-promotion",
}
REQUIRED_FEATURE_IDS = frozenset(
    {
        "presentation",
        "presentation-properties",
        "slide-master",
        "slide-layout",
        "slide",
        "theme",
        "shape-tree",
        "preset-shape",
        "custom-geometry",
        "connector",
        "group-shape",
        "text-body",
        "bullets",
        "fills",
        "effects-and-3d",
        "table",
        "image",
        "chart",
        "diagram",
        "diagram-colors",
        "diagram-data",
        "diagram-layout",
        "diagram-styles",
        "ole-embedded-object",
        "math",
        "notes",
        "comments",
        "media",
        "media-audio",
        "media-video",
        "hyperlinks-and-actions",
        "timing-and-animation",
        "transitions",
        "extensions",
        "alternate-content",
    }
)


def manifest_path_for_repository() -> Path:
    return Path(__file__).resolve().parents[2] / "evaluate/completeness_manifest.json"


def contract_path_for_repository() -> Path:
    return (
        Path(__file__).resolve().parents[2]
        / "docs/architecture/PPTX_COMPLETENESS_CONTRACT.md"
    )


def load_manifest(path: Path) -> object:
    return json.loads(path.read_text(encoding="utf-8"))


def validate_manifest(manifest: object) -> list[str]:
    if not isinstance(manifest, Mapping):
        return ["MANIFEST_ROOT_MUST_BE_OBJECT"]

    errors: list[str] = []
    if manifest.get("schema_version") != "1.0":
        errors.append("INVALID_SCHEMA_VERSION")
    if manifest.get("dimensions") != list(DIMENSIONS):
        errors.append("INVALID_DIMENSIONS")
    if manifest.get("tiers") != list(TIERS):
        errors.append("INVALID_TIERS")
    if manifest.get("stages") != list(STAGES):
        errors.append("INVALID_STAGES")
    if manifest.get("fallback_metadata_required") != list(REQUIRED_FALLBACK_METADATA):
        errors.append("INVALID_FALLBACK_METADATA_REQUIREMENTS")
    if manifest.get("exact_promotion_evidence_required") != list(
        REQUIRED_EXACT_EVIDENCE
    ):
        errors.append("INVALID_EXACT_EVIDENCE_REQUIREMENTS")
    if manifest.get("exact_promotion_gate") != EXACT_PROMOTION_GATE:
        errors.append("INVALID_EXACT_PROMOTION_GATE")

    features = manifest.get("features")
    if not isinstance(features, list):
        return [*errors, "FEATURES_MUST_BE_A_LIST"]

    feature_ids: set[str] = set()
    for feature in features:
        if not isinstance(feature, dict):
            errors.append("FEATURE_MUST_BE_AN_OBJECT")
            continue
        _validate_feature(feature, feature_ids, errors)

    for feature_id in sorted(REQUIRED_FEATURE_IDS - feature_ids):
        errors.append(f"MISSING_REQUIRED_FEATURE:{feature_id}")
    return errors


def _validate_feature(
    feature: dict[str, object], feature_ids: set[str], errors: list[str]
) -> None:
    feature_id = feature.get("id")
    if not _is_nonempty_string(feature_id):
        errors.append("MISSING_FEATURE_ID")
        return
    if feature_id in feature_ids:
        errors.append(f"DUPLICATE_FEATURE_ID:{feature_id}")
    feature_ids.add(feature_id)

    official_source = feature.get("official_source")
    if not _is_nonempty_string(official_source):
        errors.append(f"MISSING_OFFICIAL_SOURCE:{feature_id}")
    elif not _is_official_source(official_source):
        errors.append(f"UNOFFICIAL_SOURCE:{feature_id}")

    if not _is_nonempty_string(feature.get("family")):
        errors.append(f"MISSING_FEATURE_FAMILY:{feature_id}")

    ooxml = feature.get("ooxml")
    if not isinstance(ooxml, dict) or not any(
        _is_nonempty_string(ooxml.get(key))
        for key in ("qualified_name", "relationship_type")
    ):
        errors.append(f"MISSING_OOXML_REFERENCE:{feature_id}")

    fallback_policy = feature.get("fallback_policy")
    if not isinstance(fallback_policy, dict) or not _is_nonempty_string(
        fallback_policy.get("kind")
    ) or not _is_nonempty_string(fallback_policy.get("diagnostic_code")):
        errors.append(f"INVALID_FALLBACK_POLICY:{feature_id}")

    dimensions = feature.get("dimensions")
    if not isinstance(dimensions, dict):
        errors.append(f"MISSING_DIMENSIONS:{feature_id}")
        return

    for dimension in DIMENSIONS:
        dimension_value = dimensions.get(dimension)
        if not isinstance(dimension_value, dict):
            errors.append(f"MISSING_DIMENSION:{feature_id}:{dimension}")
            continue
        tier = dimension_value.get("tier")
        stage = dimension_value.get("stage")
        if tier not in TIERS:
            errors.append(f"UNCLASSIFIED_TIER:{feature_id}:{dimension}")
        if stage not in STAGES:
            errors.append(f"UNCLASSIFIED_STAGE:{feature_id}:{dimension}")
        if tier == "exact":
            _validate_exact_evidence(feature, feature_id, dimension, errors)


def _validate_exact_evidence(
    feature: dict[str, object], feature_id: str, dimension: str, errors: list[str]
) -> None:
    evidence = feature.get("exact_evidence")
    if not isinstance(evidence, dict):
        errors.append(f"EXACT_REQUIRES_POWERPOINT_EVIDENCE:{feature_id}:{dimension}")
        return
    required_scalars = (
        "oracle",
        "powerpoint_version",
        "windows_version",
        "capture_metadata",
        "fixture_bundle",
    )
    invalid_scalars = [
        field for field in required_scalars if not _is_nonempty_string(evidence.get(field))
    ]
    artifact_paths = evidence.get("artifact_paths")
    invalid_artifact_paths = not isinstance(artifact_paths, list) or not artifact_paths or any(
        not _is_nonempty_string(path) for path in artifact_paths
    )
    if (
        evidence.get("oracle") != EXACT_PROMOTION_GATE["oracle"]
        or invalid_scalars
        or invalid_artifact_paths
    ):
        errors.append(f"EXACT_REQUIRES_POWERPOINT_EVIDENCE:{feature_id}:{dimension}")


def _is_nonempty_string(value: object) -> bool:
    return isinstance(value, str) and bool(value.strip())


def _is_official_source(value: str) -> bool:
    try:
        parsed = urlsplit(value)
        port = parsed.port
    except ValueError:
        return False
    return (
        parsed.scheme == "https"
        and parsed.hostname in APPROVED_OFFICIAL_SOURCE_HOSTS
        and parsed.username is None
        and parsed.password is None
        and port is None
        and parsed.path not in ("", "/")
    )


class CompletenessManifestTests(unittest.TestCase):
    def test_rejects_unclassified_dimension_tier(self) -> None:
        manifest = _valid_manifest()
        manifest["features"][0]["dimensions"]["visual"]["tier"] = "direct"

        errors = validate_manifest(manifest)

        self.assertIn("UNCLASSIFIED_TIER:presentation:visual", errors)

    def test_rejects_missing_official_source(self) -> None:
        manifest = _valid_manifest()
        del manifest["features"][0]["official_source"]

        errors = validate_manifest(manifest)

        self.assertIn("MISSING_OFFICIAL_SOURCE:presentation", errors)

    def test_rejects_exact_dimension_without_powerpoint_evidence(self) -> None:
        manifest = _valid_manifest()
        manifest["features"][0]["dimensions"]["visual"]["tier"] = "exact"

        errors = validate_manifest(manifest)

        self.assertIn(
            "EXACT_REQUIRES_POWERPOINT_EVIDENCE:presentation:visual", errors
        )

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

    def test_rejects_invalid_root_contract_metadata(self) -> None:
        manifest = _valid_manifest()
        manifest["schema_version"] = "2.0"
        manifest["exact_promotion_gate"] = {
            "code": "wrong",
            "oracle": "browser",
            "evidence_status": "optional",
        }

        errors = validate_manifest(manifest)

        self.assertIn("INVALID_SCHEMA_VERSION", errors)
        self.assertIn("INVALID_EXACT_PROMOTION_GATE", errors)

    def test_rejects_blank_family_and_invalid_exact_evidence_shape(self) -> None:
        manifest = _valid_manifest()
        feature = manifest["features"][0]
        feature["family"] = " "
        feature["dimensions"]["visual"]["tier"] = "exact"
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
        self.assertIn(
            "EXACT_REQUIRES_POWERPOINT_EVIDENCE:presentation:visual", errors
        )

    def test_repository_manifest_is_a_valid_contract(self) -> None:
        manifest = load_manifest(manifest_path_for_repository())

        self.assertEqual(validate_manifest(manifest), [])

    def test_contract_document_names_required_policy_terms(self) -> None:
        contract = contract_path_for_repository().read_text(encoding="utf-8")

        for term in (
            "semantic preservation",
            "static visual rendering",
            "behavioral playback",
            "PowerPoint-native",
            "EXACT_REQUIRES_POWERPOINT_EVIDENCE",
            "must not silently disappear",
        ):
            self.assertIn(term, contract)

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


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path, required=True)
    args = parser.parse_args(argv)

    try:
        manifest = load_manifest(args.manifest)
    except UnicodeDecodeError:
        print("MANIFEST_TEXT_INVALID")
        return 1
    except json.JSONDecodeError:
        print("MANIFEST_JSON_INVALID")
        return 1
    except OSError:
        print("MANIFEST_READ_FAILED")
        return 1

    errors = validate_manifest(manifest)
    for error in errors:
        print(error)
    return 0 if not errors else 1


if __name__ == "__main__":
    raise SystemExit(main())
