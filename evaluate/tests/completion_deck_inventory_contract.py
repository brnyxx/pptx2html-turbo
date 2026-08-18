from __future__ import annotations

import json
import unittest
from pathlib import Path
from typing import Final


SCENARIO_CANONICAL: Final = {
    "user-defined-tags": "user-defined-tags",
    "embedded-control-persistence": "embedded-control-persistence",
    "slide-synchronization": "slide-synchronization",
    "content-part": "content-part",
    "theme-override": "theme-override",
    "thumbnail": "thumbnail",
    "custom-xml": "custom-xml",
    "additional-characteristics": "additional-characteristics",
    "bibliography": "bibliography",
    "extensions": "extensions",
    "handout-master": "handout-master",
    "rtl-text": "rtl-text",
    "text-transform-composition": "text-body",
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
    "embedded-package": "embedded-package",
    "fallback-math": "math",
    "fallback-alternate-content": "alternate-content",
    "fallback-unknown-extension": "extensions",
}


def assert_inventory(
    case: unittest.TestCase, manifest: dict[str, object], canonical_path: Path
) -> None:
    canonical = json.loads(canonical_path.read_text(encoding="utf-8"))
    canonical_ids = {row["id"] for row in canonical["features"]}
    rows = manifest["features"]
    case.assertIsInstance(rows, list)
    scenario_ids = [row["id"] for row in rows]
    case.assertEqual(len(scenario_ids), len(set(scenario_ids)), "duplicate scenarios")
    case.assertEqual(set(scenario_ids), set(SCENARIO_CANONICAL))
    for row in rows:
        scenario = row["id"]
        expected = SCENARIO_CANONICAL[scenario]
        case.assertEqual(row["completeness_feature_id"], expected, scenario)
        case.assertIn(expected, canonical_ids, scenario)
        case.assertIs(row["powerpoint_capture_required"], True)
        case.assertEqual(row["native_evidence"], {"images": [], "metadata": None})
        case.assertIn(
            row["relationship_disposition"],
            {"none", "internal", "external", "internal-audio", "internal-video"},
            scenario,
        )
    case.assertIs(manifest["powerpoint_capture_required"], True)
    case.assertEqual(manifest["native_evidence"], {"images": [], "metadata": None})
    expectations = {row["id"]: row["schema_expectation"] for row in rows}
    case.assertEqual(expectations["pattern-fill-known"], "positive")
    case.assertEqual(expectations["pattern-fill-unknown"], "negative")
    unknown = next(row for row in rows if row["id"] == "pattern-fill-unknown")
    case.assertEqual(unknown["expected_diagnostic"], "DRAWINGML_PATTERN_UNSUPPORTED")
    media_video = next(row for row in rows if row["id"] == "media-video")
    case.assertEqual(media_video["relationship_disposition"], "internal-video")
