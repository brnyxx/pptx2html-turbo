"""RFC 8785 JSON Canonicalization Scheme boundary."""

from __future__ import annotations

import json
import math
from dataclasses import dataclass
from typing import Final, assert_never

from evaluate.multiformat_schema import JsonValue

_SURROGATE_START: Final = 0xD800
_SURROGATE_END: Final = 0xDFFF


@dataclass(frozen=True, slots=True)
class JcsError(ValueError):
    """A value cannot be represented as RFC 8785 I-JSON."""

    reason: str

    def __str__(self) -> str:
        return self.reason


def canonicalize(value: JsonValue) -> bytes:
    """Return the RFC 8785 canonical UTF-8 representation of a JSON value."""
    try:
        return _serialize(value, set()).encode("utf-8")
    except AssertionError as error:
        raise JcsError(reason="unsupported JSON value") from error


def _serialize(value: JsonValue, active: set[int]) -> str:
    match value:
        case None:
            return "null"
        case bool() as boolean:
            return "true" if boolean else "false"
        case str() as string:
            return _quote(string)
        case int() | float() as number:
            return _serialize_number(number)
        case list() as items:
            identity = id(items)
            if identity in active:
                raise JcsError(reason="cyclic JSON array")
            active.add(identity)
            try:
                return "[" + ",".join(_serialize(item, active) for item in items) + "]"
            finally:
                active.remove(identity)
        case dict() as properties:
            identity = id(properties)
            if identity in active:
                raise JcsError(reason="cyclic JSON object")
            keys: list[str] = []
            for key in properties:
                match key:
                    case str() as string:
                        _validate_string(string)
                        keys.append(string)
                    case unsupported:
                        assert_never(unsupported)
            active.add(identity)
            try:
                members = (
                    _quote(key) + ":" + _serialize(properties[key], active)
                    for key in sorted(keys, key=_utf16_sort_key)
                )
                return "{" + ",".join(members) + "}"
            finally:
                active.remove(identity)
        case unsupported:
            assert_never(unsupported)


def _quote(value: str) -> str:
    _validate_string(value)
    return json.dumps(value, ensure_ascii=False)


def _validate_string(value: str) -> None:
    if any(_SURROGATE_START <= ord(character) <= _SURROGATE_END for character in value):
        raise JcsError(reason="JSON strings must not contain lone surrogates")


def _utf16_sort_key(value: str) -> bytes:
    return value.encode("utf-16-be")


def _serialize_number(value: float) -> str:
    try:
        binary64 = float(value)
    except OverflowError as error:
        raise JcsError(reason="JSON number is outside IEEE 754 binary64") from error
    if not math.isfinite(binary64):
        raise JcsError(reason="JSON numbers must be finite")
    if binary64 == 0:
        return "0"

    representation = repr(binary64).lower()
    sign = ""
    if representation.startswith("-"):
        sign = "-"
        representation = representation[1:]

    mantissa, separator, exponent_text = representation.partition("e")
    if not separator:
        return sign + mantissa.removesuffix(".0")

    exponent = int(exponent_text)
    if exponent >= 21 or exponent <= -7:
        exponent_sign = "+" if exponent >= 0 else ""
        return sign + mantissa.removesuffix(".0") + "e" + exponent_sign + str(exponent)

    digits = mantissa.replace(".", "")
    decimal_position = exponent + 1
    if decimal_position <= 0:
        return sign + "0." + ("0" * -decimal_position) + digits
    if decimal_position >= len(digits):
        return sign + digits + ("0" * (decimal_position - len(digits)))
    return sign + digits[:decimal_position] + "." + digits[decimal_position:]
