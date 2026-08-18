from __future__ import annotations

import json
import re
from dataclasses import dataclass
from pathlib import Path
from typing import cast


class AdjustmentCaseError(RuntimeError):
    pass


@dataclass(frozen=True, slots=True)
class AdjustmentSpec:
    preset: str
    key: str
    default: int
    lower: int | None
    upper: int | None
    defaults: tuple[tuple[str, int], ...]
    range_verification: str


@dataclass(frozen=True, slots=True)
class AdjustmentCase:
    preset: str
    key: str
    variant: str
    value: int
    adjustments: dict[str, int]
    range_verification: str


def load_adjustment_specs(path: Path) -> tuple[AdjustmentSpec, ...]:
    payload = _as_object(_load_json(path), "root")
    contract = _as_object(payload.get("contract"), "contract")
    expected_count = _as_int(
        contract.get("official_adjustment_count"),
        "contract.official_adjustment_count",
    )
    presets = _as_list(payload.get("presets"), "presets")
    specs: list[AdjustmentSpec] = []
    for raw_preset in presets:
        preset = _as_object(raw_preset, "preset")
        preset_name = _as_str(preset.get("name"), "preset.name")
        adjustments = _as_list(
            preset.get("adjustments"),
            f"{preset_name}.adjustments",
        )
        defaults = tuple(
            (
                _as_str(
                    _as_object(item, "adjustment").get("name"),
                    "adjustment.name",
                ),
                _default_value(_as_object(item, "adjustment")),
            )
            for item in adjustments
        )
        for raw_adjustment in adjustments:
            adjustment = _as_object(raw_adjustment, "adjustment")
            key = _as_str(adjustment.get("name"), "adjustment.name")
            lower, upper, constraint_count = _numeric_bounds(adjustment)
            if lower is not None and upper is not None:
                range_verification = "numeric-bounds"
            elif lower is not None or upper is not None:
                range_verification = "default-interpolation"
            elif constraint_count:
                range_verification = "symbolic-unverified"
            else:
                range_verification = "range-unavailable"
            specs.append(
                AdjustmentSpec(
                    preset=preset_name,
                    key=key,
                    default=_default_value(adjustment),
                    lower=lower,
                    upper=upper,
                    defaults=defaults,
                    range_verification=range_verification,
                )
            )
    if len(specs) != expected_count:
        details = f"expected={expected_count}:actual={len(specs)}"
        raise AdjustmentCaseError(
            f"ADJUSTMENT_INVENTORY_COUNT_MISMATCH:{details}"
        )
    return tuple(specs)


def build_adjustment_cases(
    specs: tuple[AdjustmentSpec, ...],
) -> tuple[AdjustmentCase, ...]:
    cases: list[AdjustmentCase] = []
    for spec in specs:
        low, high = _representative_values(spec)
        for variant, value in (
            ("low", low),
            ("default", spec.default),
            ("high", high),
        ):
            adjustments = dict(spec.defaults)
            adjustments[spec.key] = value
            cases.append(
                AdjustmentCase(
                    preset=spec.preset,
                    key=spec.key,
                    variant=variant,
                    value=value,
                    adjustments=adjustments,
                    range_verification=spec.range_verification,
                )
            )
    return tuple(cases)


def _representative_values(spec: AdjustmentSpec) -> tuple[int, int]:
    step = 5_400_000 if abs(spec.default) >= 1_000_000 else 50_000
    if spec.lower is not None and spec.upper is not None:
        candidates = _bounded_candidates(
            spec.default,
            spec.lower,
            spec.upper,
            step,
        )
    elif spec.lower is not None:
        candidates = _one_sided_candidates(
            spec.default,
            spec.lower,
            step,
            lower_bound=True,
        )
    elif spec.upper is not None:
        candidates = _one_sided_candidates(
            spec.default,
            spec.upper,
            step,
            lower_bound=False,
        )
    else:
        candidates = {spec.default - step, spec.default + step}
    alternatives = sorted(candidates - {spec.default})
    if len(alternatives) < 2:
        raise AdjustmentCaseError(
            f"ADJUSTMENT_VARIANTS_NOT_DISTINCT:{spec.preset}:{spec.key}"
        )
    return alternatives[0], alternatives[-1]


def _bounded_candidates(
    default: int,
    lower: int,
    upper: int,
    step: int,
) -> set[int]:
    if lower >= upper or not lower <= default <= upper:
        raise AdjustmentCaseError(
            f"ADJUSTMENT_NUMERIC_RANGE_INVALID:{lower}:{default}:{upper}"
        )
    candidates = {
        max(lower, default - step),
        max(lower, default - max(1, step // 2)),
        min(upper, default + max(1, step // 2)),
        min(upper, default + step),
    }
    if len(candidates - {default}) < 2:
        span = upper - lower
        candidates.update(
            lower + round(span * ratio)
            for ratio in (0.25, 0.5, 0.75)
        )
    return candidates


def _one_sided_candidates(
    default: int,
    bound: int,
    step: int,
    *,
    lower_bound: bool,
) -> set[int]:
    if lower_bound:
        if bound >= default:
            raise AdjustmentCaseError(
                f"ADJUSTMENT_LOWER_RANGE_INVALID:{bound}:{default}"
            )
        edge = max(bound, default - step)
        return {edge, edge + max(1, (default - edge) // 2)}
    if bound <= default:
        raise AdjustmentCaseError(
            f"ADJUSTMENT_UPPER_RANGE_INVALID:{default}:{bound}"
        )
    edge = min(bound, default + step)
    return {default + max(1, (edge - default) // 2), edge}


def _default_value(adjustment: dict[str, object]) -> int:
    formula = _as_str(
        adjustment.get("default_formula"),
        "adjustment.default_formula",
    )
    match = re.fullmatch(r"val (-?\d+)", formula)
    if match is None:
        raise AdjustmentCaseError(
            f"ADJUSTMENT_DEFAULT_INVALID:{formula}"
        )
    return int(match.group(1))


def _numeric_bounds(
    adjustment: dict[str, object],
) -> tuple[int | None, int | None, int]:
    constraints = _as_list(
        adjustment.get("constraints"),
        "adjustment.constraints",
    )
    lowers: list[int] = []
    uppers: list[int] = []
    for raw_constraint in constraints:
        constraint = _as_object(raw_constraint, "constraint")
        lower = _literal_int(constraint.get("minimum_formula"))
        upper = _literal_int(constraint.get("maximum_formula"))
        if lower is not None:
            lowers.append(lower)
        if upper is not None:
            uppers.append(upper)
    return (
        max(lowers) if lowers else None,
        min(uppers) if uppers else None,
        len(constraints),
    )


def _literal_int(value: object) -> int | None:
    if not isinstance(value, str) or re.fullmatch(r"-?\d+", value) is None:
        return None
    return int(value)


def _load_json(path: Path) -> object:
    try:
        return cast(object, json.loads(path.read_text(encoding="utf-8")))
    except (OSError, json.JSONDecodeError) as error:
        raise AdjustmentCaseError(
            f"ADJUSTMENT_MANIFEST_INVALID:{path}:{error}"
        ) from error


def _as_object(value: object, field: str) -> dict[str, object]:
    if not isinstance(value, dict):
        raise AdjustmentCaseError(f"ADJUSTMENT_FIELD_INVALID:{field}")
    return cast(dict[str, object], value)


def _as_list(value: object, field: str) -> list[object]:
    if not isinstance(value, list):
        raise AdjustmentCaseError(f"ADJUSTMENT_FIELD_INVALID:{field}")
    return cast(list[object], value)


def _as_str(value: object, field: str) -> str:
    if not isinstance(value, str) or not value:
        raise AdjustmentCaseError(f"ADJUSTMENT_FIELD_INVALID:{field}")
    return value


def _as_int(value: object, field: str) -> int:
    if not isinstance(value, int) or isinstance(value, bool):
        raise AdjustmentCaseError(f"ADJUSTMENT_FIELD_INVALID:{field}")
    return value
