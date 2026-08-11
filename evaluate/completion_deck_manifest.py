from __future__ import annotations

import json
from pathlib import Path
from typing import TypeAlias
from xml.sax.saxutils import quoteattr

if __package__:
    from .check_preset_adjustments import (
        ContractError as AdjustmentContractError,
        check_repository,
    )
else:
    from check_preset_adjustments import (
        ContractError as AdjustmentContractError,
        check_repository,
    )

JsonValue: TypeAlias = (
    str | int | bool | None | list["JsonValue"] | dict[str, "JsonValue"]
)
JsonMap: TypeAlias = dict[str, JsonValue]
CANONICAL_MANIFEST = Path(__file__).with_name("preset_adjustments.json")
REPO_ROOT = Path(__file__).resolve().parents[1]


class ContractError(Exception):
    pass


S = "ppt/slides/slide1.xml"


def load_adjustments(path: Path) -> tuple[tuple[str, ...], dict[str, JsonMap]]:
    parsed = _read_manifest(path)
    canonical = _read_manifest(CANONICAL_MANIFEST)
    try:
        report = check_repository(REPO_ROOT, manifest_path=path)
    except AdjustmentContractError as error:
        raise ContractError(f"ADJUSTMENT_CONTRACT_INVALID {error}") from error
    if report.get("ok") is not True:
        raise ContractError("ADJUSTMENT_DISPATCHER_MISMATCH unknown consumed key")
    if parsed != canonical:
        raise ContractError("ADJUSTMENT_CONTRACT_MISMATCH canonical manifest differs")
    names, presets = parsed.get("official_preset_names"), parsed.get("presets")
    if not isinstance(names, list) or not isinstance(presets, list):
        raise ContractError("ADJUSTMENT_INVENTORY_MISMATCH invalid root")
    return tuple(str(name) for name in names), {
        str(row["name"]): row for row in presets if isinstance(row, dict)
    }


def _read_manifest(path: Path) -> JsonMap:
    try:
        parsed = json.loads(path.read_text(encoding="utf-8"))
    except (FileNotFoundError, OSError, json.JSONDecodeError) as error:
        raise ContractError(f"ADJUSTMENT_MANIFEST_ERROR path={path}") from error
    if not isinstance(parsed, dict):
        raise ContractError("ADJUSTMENT_INVENTORY_MISMATCH invalid root")
    return parsed


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
            if not isinstance(raw_value, str):
                raise ContractError(f"ADJUSTMENT_CASE_UNAVAILABLE preset={preset}")
            value = raw_value
            formula = value if value.startswith("val ") else f"val {value}"
            key = adjustment.get("name")
            source_status = adjustment.get("source_status")
            if not isinstance(key, str) or not isinstance(source_status, str):
                raise ContractError(f"ADJUSTMENT_CASE_UNAVAILABLE preset={preset}")
            token = f"<a:prstGeom prst={quoteattr(preset)}><a:avLst><a:gd name={quoteattr(key)} fmla={quoteattr(formula)}/></a:avLst>"
            shape_name = f"adjustment-{bundle}-{kind}"
            shape_id = 20 + len(shapes)
            shapes.append(
                f"<p:sp><p:nvSpPr><p:cNvPr id={quoteattr(str(shape_id))} name={quoteattr(shape_name)}/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr>{token}</a:prstGeom></p:spPr></p:sp>"
            )
            cases.append(
                {
                    "bundle": bundle,
                    "preset": preset,
                    "key": key,
                    "kind": kind,
                    "value_or_formula": raw_value,
                    "source_field": source_field,
                    "source_status": source_status,
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
