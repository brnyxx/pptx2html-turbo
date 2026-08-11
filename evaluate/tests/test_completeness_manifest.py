from __future__ import annotations

import argparse
import json
import unittest
from collections.abc import Mapping, Sequence
from pathlib import Path


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


def load_manifest(path: Path) -> dict[str, object]:
    return json.loads(path.read_text(encoding="utf-8"))


def validate_manifest(manifest: Mapping[str, object]) -> list[str]:
    errors: list[str] = []
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
    if not isinstance(feature_id, str) or not feature_id:
        errors.append("MISSING_FEATURE_ID")
        return
    if feature_id in feature_ids:
        errors.append(f"DUPLICATE_FEATURE_ID:{feature_id}")
    feature_ids.add(feature_id)

    if not isinstance(feature.get("official_source"), str) or not feature[
        "official_source"
    ].startswith("https://"):
        errors.append(f"MISSING_OFFICIAL_SOURCE:{feature_id}")

    ooxml = feature.get("ooxml")
    if not isinstance(ooxml, dict) or not any(
        isinstance(ooxml.get(key), str) and ooxml[key]
        for key in ("qualified_name", "relationship_type")
    ):
        errors.append(f"MISSING_OOXML_REFERENCE:{feature_id}")

    fallback_policy = feature.get("fallback_policy")
    if not isinstance(fallback_policy, dict) or not isinstance(
        fallback_policy.get("kind"), str
    ) or not isinstance(fallback_policy.get("diagnostic_code"), str):
        errors.append(f"MISSING_FALLBACK_POLICY:{feature_id}")

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
    missing_evidence = [
        field for field in REQUIRED_EXACT_EVIDENCE if not evidence.get(field)
    ]
    if evidence.get("oracle") != "PowerPoint-native" or missing_evidence:
        errors.append(f"EXACT_REQUIRES_POWERPOINT_EVIDENCE:{feature_id}:{dimension}")


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


def _valid_manifest() -> dict[str, object]:
    feature = {
        "id": "presentation",
        "official_source": "https://example.invalid/official",
        "ooxml": {"qualified_name": "p:presentation"},
        "fallback_policy": {
            "kind": "diagnostic-placeholder",
            "diagnostic_code": "PPTX_COMPLETENESS_FALLBACK",
        },
        "dimensions": {
            dimension: {"tier": "fallback", "stage": "parsed"}
            for dimension in DIMENSIONS
        },
    }
    remaining_features = [
        {
            **feature,
            "id": feature_id,
            "ooxml": {"qualified_name": f"p:{feature_id}"},
        }
        for feature_id in sorted(REQUIRED_FEATURE_IDS - {"presentation"})
    ]
    return {
        "dimensions": list(DIMENSIONS),
        "tiers": list(TIERS),
        "stages": list(STAGES),
        "fallback_metadata_required": list(REQUIRED_FALLBACK_METADATA),
        "exact_promotion_evidence_required": list(REQUIRED_EXACT_EVIDENCE),
        "features": [feature, *remaining_features],
    }


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path, required=True)
    args = parser.parse_args(argv)

    errors = validate_manifest(load_manifest(args.manifest))
    for error in errors:
        print(error)
    return 0 if not errors else 1


if __name__ == "__main__":
    raise SystemExit(main())
