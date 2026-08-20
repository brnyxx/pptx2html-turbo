from __future__ import annotations

import hashlib
import json
from pathlib import Path
from typing import cast

type JsonValue = (
    None | bool | int | float | str | list[JsonValue] | dict[str, JsonValue]
)


def read_object(path: Path) -> dict[str, JsonValue]:
    value = cast(JsonValue, json.loads(path.read_text(encoding="utf-8")))
    if not isinstance(value, dict):
        raise TypeError("expected a JSON object")
    return value


def object_value(values: dict[str, JsonValue], field: str) -> dict[str, JsonValue]:
    value = values.get(field)
    if not isinstance(value, dict):
        raise TypeError(f"{field} must be an object")
    return value


def string_list(values: dict[str, JsonValue], field: str) -> list[str]:
    value = values.get(field)
    if not isinstance(value, list) or not all(isinstance(item, str) for item in value):
        raise TypeError(f"{field} must be a string array")
    return cast(list[str], value)


def string_value(values: dict[str, JsonValue], field: str) -> str:
    value = values.get(field)
    if not isinstance(value, str) or not value:
        raise TypeError(f"{field} must be a non-empty string")
    return value


def sha256_value(values: dict[str, JsonValue], field: str) -> str:
    value = string_value(values, field)
    if len(value) != 64 or any(
        character not in "0123456789abcdef" for character in value
    ):
        raise ValueError(f"{field} must be a lowercase SHA-256 digest")
    return value


def integer_value(values: dict[str, JsonValue], field: str) -> int:
    value = values.get(field)
    if not isinstance(value, int) or isinstance(value, bool):
        raise TypeError(f"{field} must be an integer")
    return value


def number_value(values: dict[str, JsonValue], field: str) -> float:
    value = values.get(field)
    if not isinstance(value, int | float) or isinstance(value, bool):
        raise TypeError(f"{field} must be numeric")
    return float(value)


def boolean_value(values: dict[str, JsonValue], field: str) -> bool:
    value = values.get(field)
    if not isinstance(value, bool):
        raise TypeError(f"{field} must be a boolean")
    return value


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()
