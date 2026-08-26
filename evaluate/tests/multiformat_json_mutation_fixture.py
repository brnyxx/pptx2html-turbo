from __future__ import annotations

from typing import assert_never

from evaluate.multiformat_schema import JsonValue


def set_json_path(
    value: dict[str, JsonValue], path: str, replacement: JsonValue
) -> None:
    """Mutate one typed JSON path for an adversarial fixture."""
    current: JsonValue = value
    parts = path.split(".")
    for part in parts[:-1]:
        current = _child(current, part)
    _assign(current, parts[-1], replacement)


def _child(current: JsonValue, part: str) -> JsonValue:
    match current:
        case list():
            return current[int(part)]
        case dict():
            return current[part]
        case None | bool() | int() | float() | str():
            raise TypeError(part)
        case unreachable:
            assert_never(unreachable)


def _assign(current: JsonValue, part: str, replacement: JsonValue) -> None:
    match current:
        case list():
            current[int(part)] = replacement
        case dict():
            current[part] = replacement
        case None | bool() | int() | float() | str():
            raise TypeError(part)
        case unreachable:
            assert_never(unreachable)
