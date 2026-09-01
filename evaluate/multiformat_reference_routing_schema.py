from __future__ import annotations

from evaluate.multiformat_schema import JsonValue


def require_keys(
    values: dict[str, JsonValue], expected: set[str], context: str
) -> None:
    if set(values) != expected:
        raise ValueError(f"{context} keys do not match the schema")


def mapping(value: JsonValue, field: str) -> dict[str, JsonValue]:
    if not isinstance(value, dict):
        raise ValueError(f"{field} must be an object")
    return value


def array(values: dict[str, JsonValue], field: str) -> list[JsonValue]:
    value = values.get(field)
    if not isinstance(value, list):
        raise ValueError(f"{field} must be an array")
    return value


def string_array(values: dict[str, JsonValue], field: str) -> tuple[str, ...]:
    items = array(values, field)
    if not all(isinstance(item, str) for item in items):
        raise ValueError(f"{field} must be a string array")
    return tuple(item for item in items if isinstance(item, str))


def string(values: dict[str, JsonValue], field: str) -> str:
    value = values.get(field)
    if not isinstance(value, str) or not value:
        raise ValueError(f"{field} must be a non-empty string")
    return value


def integer(values: dict[str, JsonValue], field: str) -> int:
    value = values.get(field)
    if not isinstance(value, int) or isinstance(value, bool):
        raise ValueError(f"{field} must be an integer")
    return value


def boolean(values: dict[str, JsonValue], field: str) -> bool:
    value = values.get(field)
    if not isinstance(value, bool):
        raise ValueError(f"{field} must be a boolean")
    return value
