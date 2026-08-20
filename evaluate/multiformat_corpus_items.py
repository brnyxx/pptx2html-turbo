from __future__ import annotations

import unicodedata
import re
from urllib.parse import urlsplit

from evaluate.multiformat_corpus_types import CorpusError
from evaluate.multiformat_schema import JsonValue, integer_value

BACKGROUND = re.compile(r"^#[0-9a-f]{6}$")


def track_items(
    track: dict[str, JsonValue],
    expected_count: int,
    name: str,
) -> list[dict[str, JsonValue]]:
    require_keys(track, {"expected_count", "items"}, f"{name}.track")
    if integer_value(track, "expected_count") != expected_count:
        raise CorpusError(f"{name}.expected_count", str(expected_count))
    return object_list(track, "items", f"{name}.items")


def object_list(
    values: dict[str, JsonValue],
    field: str,
    reason: str,
) -> list[dict[str, JsonValue]]:
    value = values.get(field)
    if not isinstance(value, list):
        raise CorpusError(reason, "must be an object array")
    result: list[dict[str, JsonValue]] = []
    for item in value:
        if not isinstance(item, dict):
            raise CorpusError(reason, "must be an object array")
        result.append(item)
    return result


def require_keys(
    values: dict[str, JsonValue],
    expected: set[str],
    reason: str,
) -> None:
    if set(values) != expected:
        raise CorpusError(reason, "unexpected object fields")


def add_unique(values: set[str], value: str, reason: str) -> None:
    if value in values:
        raise CorpusError(reason, value)
    values.add(value)


def canonical_identity(value: str, reason: str) -> str:
    canonical = unicodedata.normalize("NFKC", value).strip().casefold()
    if not canonical:
        raise CorpusError(reason, value)
    return canonical


def canonical_source_uri(value: str) -> str:
    parsed = urlsplit(value)
    if (
        parsed.scheme.casefold() == "https"
        and parsed.hostname
        and parsed.username is None
        and parsed.password is None
    ):
        return parsed._replace(
            scheme="https",
            netloc=parsed.netloc.casefold(),
        ).geturl()
    if parsed.scheme.casefold() == "urn" and parsed.path:
        return unicodedata.normalize("NFKC", value).casefold()
    raise CorpusError("blind.source_uri", value)


def validate_background(value: str, reason: str) -> None:
    if BACKGROUND.fullmatch(value) is None:
        raise CorpusError(reason, value)
