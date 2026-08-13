from __future__ import annotations

import json
from pathlib import Path
from typing import Final, Protocol

if __package__:
    from .completion_deck_features import SchemaExpectation
    from .completion_deck_manifest import ContractError
else:
    from completion_deck_features import SchemaExpectation
    from completion_deck_manifest import ContractError


class FeatureLike(Protocol):
    task: int
    deck: str
    feature_id: str
    part: str
    token: str
    schema_expectation: SchemaExpectation
    expected_diagnostic: str | None
    relationship_disposition: str


SCENARIO_CANONICAL: Final = {
    "additional-characteristics": "additional-characteristics",
    "bibliography": "bibliography",
    "extensions": "extensions",
    "handout-master": "handout-master",
    "rtl-text": "rtl-text",
    "adjustment-basic": "preset-shape",
    "adjustment-arrows": "preset-shape",
    "adjustment-remaining": "preset-shape",
    "custom-geometry-unknown-formula": "custom-geometry",
    "pattern-fill-known": "pattern-fill",
    "pattern-fill-unknown": "pattern-fill",
    "picture-bullet-embedded": "picture-bullets",
    "picture-bullet-missing": "picture-bullets",
    "table-style-regions": "table-style",
    "table-style-missing": "table-style",
    "action-external": "shape-hyperlink-and-action",
    "action-internal": "shape-hyperlink-and-action",
    "action-unsafe": "shape-hyperlink-and-action",
    "action-table-frame": "shape-hyperlink-and-action",
    "action-group": "shape-hyperlink-and-action",
    "notes-slide": "notes",
    "comments-legacy": "comments",
    "comments-modern": "comments",
    "comment-author-missing": "comment-authors",
    "reflection": "reflection-and-3d",
    "drawingml-3d-fallback": "reflection-and-3d",
    "media-audio": "media-audio",
    "media-video": "media-video",
    "media-unsupported": "media-audio",
    "transition-cut": "transitions",
    "transition-fade": "transitions",
    "animation-bounded": "timing-and-animation",
    "animation-unsupported": "timing-and-animation",
    "chart-direct": "chart-direct-subset",
    "chart-preview-fallback": "chart-preview-fallback",
    "chart-placeholder": "chart-placeholder-fallback",
    "fallback-smartart": "diagram",
    "fallback-ole": "ole-embedded-object",
    "fallback-math": "math",
    "fallback-alternate-content": "alternate-content",
    "fallback-unknown-extension": "extensions",
}
SCHEMA_NEGATIVES: Final = {
    "pattern-fill-unknown": "DRAWINGML_PATTERN_UNSUPPORTED",
}


def validate_features(features: tuple[FeatureLike, ...], canonical_path: Path) -> None:
    scenario_ids = [feature.feature_id for feature in features]
    duplicates = sorted({item for item in scenario_ids if scenario_ids.count(item) > 1})
    if duplicates:
        raise ContractError(f"COMPLETION_SCENARIO_DUPLICATE ids={','.join(duplicates)}")
    missing = sorted(set(SCENARIO_CANONICAL) - set(scenario_ids))
    extra = sorted(set(scenario_ids) - set(SCENARIO_CANONICAL))
    if missing or extra:
        raise ContractError(
            f"COMPLETION_SCENARIO_MISMATCH missing={','.join(missing)} extra={','.join(extra)}"
        )
    for feature in features:
        if feature.relationship_disposition not in {
            "none",
            "internal",
            "external",
            "internal-audio",
            "internal-video",
        }:
            raise ContractError(
                "COMPLETION_RELATIONSHIP_DISPOSITION_INVALID "
                f"id={feature.feature_id} value={feature.relationship_disposition}"
            )
        try:
            expectation = SchemaExpectation(feature.schema_expectation)
        except ValueError as error:
            raise ContractError(
                "COMPLETION_SCHEMA_EXPECTATION_INVALID "
                f"id={feature.feature_id} value={feature.schema_expectation}"
            ) from error
        expected = (
            (SchemaExpectation.NEGATIVE, SCHEMA_NEGATIVES[feature.feature_id])
            if feature.feature_id in SCHEMA_NEGATIVES
            else (SchemaExpectation.POSITIVE, None)
        )
        if (expectation, feature.expected_diagnostic) != expected:
            raise ContractError(
                "COMPLETION_SCHEMA_CONTRACT "
                f"id={feature.feature_id} expectation={expectation} "
                f"diagnostic={feature.expected_diagnostic}"
            )
    try:
        payload = json.loads(canonical_path.read_text(encoding="utf-8"))
        rows = payload["features"]
        canonical_ids = [row["id"] for row in rows]
    except (OSError, json.JSONDecodeError, KeyError, TypeError) as error:
        raise ContractError(
            f"COMPLETENESS_CANONICAL_ERROR path={canonical_path}"
        ) from error
    duplicate_canonical = sorted(
        {item for item in canonical_ids if canonical_ids.count(item) > 1}
    )
    if duplicate_canonical:
        raise ContractError(
            f"COMPLETENESS_CANONICAL_DUPLICATE ids={','.join(duplicate_canonical)}"
        )
    unknown = sorted(set(SCENARIO_CANONICAL.values()) - set(canonical_ids))
    if unknown:
        raise ContractError(f"COMPLETENESS_CANONICAL_UNKNOWN ids={','.join(unknown)}")
