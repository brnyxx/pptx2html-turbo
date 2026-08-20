from __future__ import annotations

import json
from pathlib import Path
from typing import cast

from evaluate.multiformat_schema import JsonValue

MAX_JSON_BYTES = 32 * 1024 * 1024


class StrictJsonError(ValueError):
    pass


def read_strict_object(path: Path) -> dict[str, JsonValue]:
    try:
        if not 0 < path.stat().st_size <= MAX_JSON_BYTES:
            raise StrictJsonError("JSON file exceeds the bounded size")
        value = cast(
            JsonValue,
            json.loads(
                path.read_text(encoding="utf-8"),
                object_pairs_hook=_unique_object,
                parse_constant=_reject_constant,
            ),
        )
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise StrictJsonError(path.as_posix()) from error
    if not isinstance(value, dict):
        raise StrictJsonError("expected a JSON object")
    return value


def _unique_object(pairs: list[tuple[str, JsonValue]]) -> dict[str, JsonValue]:
    result: dict[str, JsonValue] = {}
    for key, value in pairs:
        if key in result:
            raise StrictJsonError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def _reject_constant(value: str) -> JsonValue:
    raise StrictJsonError(f"invalid JSON numeric constant: {value}")
