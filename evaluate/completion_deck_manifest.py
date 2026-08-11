from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import TypeAlias

JsonValue: TypeAlias = (
    str | int | bool | None | list["JsonValue"] | dict[str, "JsonValue"]
)
JsonMap: TypeAlias = dict[str, JsonValue]


class ContractError(Exception):
    pass


@dataclass(frozen=True, slots=True)
class FeatureSpec:
    task: int
    deck: str
    feature_id: str
    part: str
    token: str


S = "ppt/slides/slide1.xml"
SR = "ppt/slides/_rels/slide1.xml.rels"


def _f(task: int, deck: str, feature_id: str, token: str, part: str = S) -> FeatureSpec:
    return FeatureSpec(task, deck, feature_id, part, token)


FEATURES = (
    _f(8, "patterns", "adjustment-basic", '<a:prstGeom prst="roundRect"><a:avLst>'),
    _f(9, "patterns", "adjustment-arrows", '<a:prstGeom prst="rightArrow"><a:avLst>'),
    _f(10, "patterns", "adjustment-remaining", '<a:prstGeom prst="wave"><a:avLst>'),
    _f(
        10,
        "patterns",
        "custom-geometry-unknown-formula",
        '<a:gd name="unknownGuide" fmla="unknownOp 1 2"/>',
    ),
    _f(12, "patterns", "pattern-fill-known", '<a:pattFill prst="pct5">'),
    _f(
        12,
        "patterns",
        "pattern-fill-unknown",
        '<a:pattFill prst="unknownFuturePattern">',
    ),
    _f(
        13, "picture-bullets", "picture-bullet-embedded", '<a:blip r:embed="rIdImage"/>'
    ),
    _f(
        13,
        "picture-bullets",
        "picture-bullet-missing",
        '<a:blip r:embed="rIdMissing"/>',
    ),
    _f(
        14,
        "table-styles",
        "table-style-regions",
        "{11111111-1111-1111-1111-111111111111}",
    ),
    _f(
        14,
        "table-styles",
        "table-style-missing",
        "{22222222-2222-2222-2222-222222222222}",
    ),
    _f(15, "actions", "action-external", '<Relationship Id="rIdExternal"', SR),
    _f(
        15,
        "actions",
        "action-internal",
        '<a:hlinkClick action="ppaction://hlinkshowjump?jump=nextslide"/>',
    ),
    _f(15, "actions", "action-unsafe", '<Relationship Id="rIdUnsafe"', SR),
    _f(
        16,
        "notes-comments",
        "notes-slide",
        "<p:notes ",
        "ppt/notesSlides/notesSlide1.xml",
    ),
    _f(
        16,
        "notes-comments",
        "comments-legacy",
        "<p:text>LEGACY_COMMENT</p:text>",
        "ppt/comments/comment1.xml",
    ),
    _f(
        16,
        "notes-comments",
        "comments-modern",
        "<p188:cm id=",
        "ppt/comments/modernComment1.xml",
    ),
    _f(
        16,
        "notes-comments",
        "comment-author-missing",
        '<p:cm authorId="404"',
        "ppt/comments/comment1.xml",
    ),
    _f(17, "reflection-3d", "reflection", "<a:reflection "),
    _f(17, "reflection-3d", "drawingml-3d-fallback", "<a:scene3d>"),
    _f(18, "media", "media-audio", '<a:audioFile r:link="rIdAudio"/>'),
    _f(18, "media", "media-video", '<a:videoFile r:link="rIdVideo"/>'),
    _f(18, "media", "media-unsupported", '<a:audioFile r:link="rIdUnsupported"/>'),
    _f(
        19,
        "timing-transitions",
        "transition-cut",
        '<p:transition spd="slow"><p:cut/>',
        "ppt/slides/slide2.xml",
    ),
    _f(
        19,
        "timing-transitions",
        "transition-fade",
        '<p:transition spd="slow"><p:fade/>',
    ),
    _f(
        19,
        "timing-transitions",
        "animation-bounded",
        '<p:animEffect transition="in" filter="fade">',
    ),
    _f(
        19,
        "timing-transitions",
        "animation-unsupported",
        '<p:animMotion origin="layout"',
    ),
    _f(20, "charts", "chart-direct", '<c:chart r:id="rIdChartDirect"/>'),
    _f(
        20,
        "charts",
        "chart-preview-fallback",
        '<Relationship Id="rIdPreviewImage"',
        "ppt/charts/_rels/chart2.xml.rels",
    ),
    _f(20, "charts", "chart-placeholder", "<c:stockChart/>", "ppt/charts/chart3.xml"),
    _f(21, "fallback-domains", "fallback-smartart", "<a:relIds "),
    _f(21, "fallback-domains", "fallback-ole", '<p:oleObj r:id="rIdOle"'),
    _f(
        21,
        "fallback-domains",
        "fallback-math",
        '<m:oMath xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math">',
    ),
    _f(21, "fallback-domains", "fallback-alternate-content", "<mc:AlternateContent>"),
    _f(
        21,
        "fallback-domains",
        "fallback-unknown-extension",
        '<unknown:payload xmlns:unknown="urn:pptx2html:test:unknown"',
    ),
)


def load_adjustments(path: Path) -> tuple[tuple[str, ...], dict[str, JsonMap]]:
    try:
        parsed = json.loads(path.read_text(encoding="utf-8"))
    except (FileNotFoundError, OSError, json.JSONDecodeError) as error:
        raise ContractError(f"ADJUSTMENT_MANIFEST_ERROR path={path}") from error
    if not isinstance(parsed, dict):
        raise ContractError("ADJUSTMENT_INVENTORY_MISMATCH invalid root")
    names, presets = parsed.get("official_preset_names"), parsed.get("presets")
    if not isinstance(names, list) or not isinstance(presets, list):
        raise ContractError("ADJUSTMENT_INVENTORY_MISMATCH invalid root")
    if any(not isinstance(name, str) for name in names) or any(
        not isinstance(row, dict) for row in presets
    ):
        raise ContractError("ADJUSTMENT_INVENTORY_MISMATCH invalid rows")
    row_names = [row.get("name") for row in presets]
    if (
        len(names) != 187
        or len(set(names)) != 187
        or len(presets) != 187
        or any(not isinstance(name, str) for name in row_names)
        or len(set(row_names)) != 187
        or set(row_names) != set(names)
    ):
        raise ContractError(
            "ADJUSTMENT_INVENTORY_MISMATCH missing extra or duplicate preset"
        )
    return tuple(names), {row["name"]: row for row in presets}


def adjustment_cases(rows: dict[str, JsonMap]) -> tuple[list[JsonMap], str]:
    cases: list[JsonMap] = []
    shapes: list[str] = []
    for bundle, preset in (
        ("basic", "roundRect"),
        ("arrows", "rightArrow"),
        ("remaining", "wave"),
    ):
        adjustment = _first_adjustment(rows, preset)
        default = adjustment.get("default_formula")
        if not isinstance(default, str):
            raise ContractError(f"ADJUSTMENT_CASE_UNAVAILABLE preset={preset}")
        constraint = _first_constraint(adjustment)
        values = (
            ("default", default, "default_formula"),
            (
                "lower",
                constraint.get("minimum_formula") or default,
                "constraints.minimum_formula",
            ),
            (
                "upper",
                constraint.get("maximum_formula") or default,
                "constraints.maximum_formula",
            ),
            ("representative", default, "default_formula"),
        )
        for kind, raw_value, source_field in values:
            value = str(raw_value)
            formula = value if value.startswith("val ") else f"val {value}"
            key = str(adjustment["name"])
            token = f'<a:prstGeom prst="{preset}"><a:avLst><a:gd name="{key}" fmla="{formula}"/></a:avLst>'
            shape_name = f"adjustment-{bundle}-{kind}"
            shape_id = 20 + len(shapes)
            shapes.append(
                f'<p:sp><p:nvSpPr><p:cNvPr id="{shape_id}" name="{shape_name}"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr>{token}</a:prstGeom></p:spPr></p:sp>'
            )
            cases.append(
                {
                    "bundle": bundle,
                    "preset": preset,
                    "key": key,
                    "kind": kind,
                    "value_or_formula": raw_value,
                    "source_field": source_field,
                    "source_status": adjustment.get("source_status"),
                    "stimulus": {"part": S, "token": token},
                    "expected_pixels": None,
                }
            )
    return cases, "".join(shapes)


def _first_adjustment(rows: dict[str, JsonMap], preset: str) -> JsonMap:
    row = rows.get(preset)
    adjustments = row.get("adjustments") if row else None
    if (
        not isinstance(adjustments, list)
        or not adjustments
        or not isinstance(adjustments[0], dict)
    ):
        raise ContractError(f"ADJUSTMENT_CASE_UNAVAILABLE preset={preset}")
    return adjustments[0]


def _first_constraint(adjustment: JsonMap) -> JsonMap:
    constraints = adjustment.get("constraints")
    if (
        isinstance(constraints, list)
        and constraints
        and isinstance(constraints[0], dict)
    ):
        return constraints[0]
    return {}
