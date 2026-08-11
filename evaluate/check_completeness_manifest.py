from __future__ import annotations

import argparse
import json
import re
from collections.abc import Mapping, Sequence
from pathlib import Path
from urllib.parse import urlsplit


CONTRACT_SCOPE = (
    "target disposition; this manifest does not claim current direct support "
    "without feature evidence"
)
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
FEATURE_ID_PATTERN = re.compile(r"[a-z][a-z0-9]*(?:-[a-z0-9]+)*")
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


def load_manifest(path: Path) -> object:
    return json.loads(path.read_text(encoding="utf-8"))


def validate_manifest(manifest: object) -> list[str]:
    if not isinstance(manifest, Mapping):
        return ["MANIFEST_ROOT_MUST_BE_OBJECT"]

    errors: list[str] = []
    if manifest.get("schema_version") != "1.0":
        errors.append("INVALID_SCHEMA_VERSION")
    if manifest.get("contract_scope") != CONTRACT_SCOPE:
        errors.append("INVALID_CONTRACT_SCOPE")
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
    if FEATURE_ID_PATTERN.fullmatch(feature_id) is None:
        errors.append(f"INVALID_FEATURE_ID:{feature_id}")
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
    if (
        not isinstance(fallback_policy, dict)
        or not _is_nonempty_string(fallback_policy.get("kind"))
        or not _is_nonempty_string(fallback_policy.get("diagnostic_code"))
    ):
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
        field
        for field in required_scalars
        if not _is_nonempty_string(evidence.get(field))
    ]
    artifact_paths = evidence.get("artifact_paths")
    invalid_artifact_paths = (
        not isinstance(artifact_paths, list)
        or not artifact_paths
        or any(not _is_nonempty_string(path) for path in artifact_paths)
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
