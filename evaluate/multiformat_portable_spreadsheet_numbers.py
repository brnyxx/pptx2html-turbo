"""Classification of ECMA-376 number format codes.

Mirrors `crates/document2html-core/src/spreadsheet/number.rs`. Only a bounded
subset is reproduced: decimal precision, thousands grouping, a leading or
trailing currency literal, percentages and ISO dates. Any other code is
reported as unsupported so attribution fails closed.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Final

GENERAL: Final = "general"
DECIMAL: Final = "decimal"
PERCENT: Final = "percent"
ISO_DATE: Final = "iso_date"
ISO_DATE_TIME: Final = "iso_date_time"
UNSUPPORTED: Final = "unsupported"

_PLACEHOLDERS: Final = frozenset("0#?")
_NUMERIC_BODY: Final = frozenset("0#.,-+ ?")
_PERCENT_BODY: Final = frozenset("0#., ?")
_CURRENCY_CHARS: Final = frozenset("$\u00a4\u20ac\u00a3\u00a5\u20a9")


@dataclass(frozen=True, slots=True)
class DecimalFormat:
    decimals: int = 0
    grouped: bool = False
    prefix: str = ""
    suffix: str = ""
    parenthesized_negative: bool = False


@dataclass(frozen=True, slots=True)
class ResolvedFormat:
    kind: str
    decimals: int = 0
    spec: DecimalFormat = field(default_factory=DecimalFormat)


GENERAL_FORMAT: Final = ResolvedFormat(GENERAL)
UNSUPPORTED_FORMAT: Final = ResolvedFormat(UNSUPPORTED)


def _plain(decimals: int, grouped: bool) -> ResolvedFormat:
    return ResolvedFormat(
        DECIMAL, spec=DecimalFormat(decimals=decimals, grouped=grouped)
    )


def _accounting(decimals: int) -> ResolvedFormat:
    return ResolvedFormat(
        DECIMAL,
        spec=DecimalFormat(
            decimals=decimals, grouped=True, parenthesized_negative=True
        ),
    )


# Built-in numFmt ids reproduced here, matching the Rust core exactly.
_BUILTIN: Final[dict[str, ResolvedFormat]] = {
    "0": GENERAL_FORMAT,
    "49": GENERAL_FORMAT,
    "1": _plain(0, False),
    "2": _plain(2, False),
    "3": _plain(0, True),
    "4": _plain(2, True),
    "9": ResolvedFormat(PERCENT, decimals=0),
    "10": ResolvedFormat(PERCENT, decimals=2),
    "14": ResolvedFormat(ISO_DATE),
    "15": ResolvedFormat(ISO_DATE),
    "16": ResolvedFormat(ISO_DATE),
    "17": ResolvedFormat(ISO_DATE),
    "22": ResolvedFormat(ISO_DATE_TIME),
    "37": _accounting(0),
    "38": _accounting(0),
    "39": _accounting(2),
    "40": _accounting(2),
}


def builtin_format(identity: str) -> ResolvedFormat:
    return _BUILTIN.get(identity, UNSUPPORTED_FORMAT)


def classify_format_code(code: str) -> ResolvedFormat:
    """Classifies a custom format code."""
    sections = code.split(";")
    positive = sections[0]
    parenthesized_negative = False
    if len(sections) > 2:
        return UNSUPPORTED_FORMAT
    if len(sections) == 2:
        trimmed = sections[1].strip()
        # Parentheses are the accounting convention; any other rewrite of the
        # negative form is not reproduced.
        if not (trimmed.startswith("(") and trimmed.endswith(")")):
            return UNSUPPORTED_FORMAT
        parenthesized_negative = True

    prefix, suffix, body = _split_literals(positive)
    body = body.strip()
    if not body:
        return UNSUPPORTED_FORMAT
    if "%" in body:
        return _percent_format(body, parenthesized_negative)
    lowered = body.lower()
    if any(marker in lowered for marker in "ydm"):
        if prefix or suffix:
            return UNSUPPORTED_FORMAT
        if any(marker in lowered for marker in "hs"):
            return ResolvedFormat(ISO_DATE_TIME)
        return ResolvedFormat(ISO_DATE)
    return _numeric_format(body, prefix, suffix, parenthesized_negative)


def _percent_format(body: str, parenthesized_negative: bool) -> ResolvedFormat:
    if parenthesized_negative:
        return UNSUPPORTED_FORMAT
    numeric = body.replace("%", "")
    if not all(character in _PERCENT_BODY for character in numeric):
        return UNSUPPORTED_FORMAT
    decimals = _decimals_of(numeric)
    if decimals is None:
        return UNSUPPORTED_FORMAT
    return ResolvedFormat(PERCENT, decimals=decimals)


def _numeric_format(
    body: str,
    prefix: str,
    suffix: str,
    parenthesized_negative: bool,
) -> ResolvedFormat:
    if not all(character in _NUMERIC_BODY for character in body):
        return UNSUPPORTED_FORMAT
    if "0" not in body and "#" not in body:
        return UNSUPPORTED_FORMAT
    integer = body.split(".", 1)[0]
    decimals = _decimals_of(body)
    if decimals is None:
        return UNSUPPORTED_FORMAT
    return ResolvedFormat(
        DECIMAL,
        spec=DecimalFormat(
            decimals=decimals,
            grouped="," in integer,
            prefix=prefix,
            suffix=suffix,
            parenthesized_negative=parenthesized_negative,
        ),
    )


def _decimals_of(code: str) -> int | None:
    """Fixed fraction digits, or None when optional placeholders are used.

    `#` and `?` suppress or pad trailing zeros, which is not reproduced here.
    """
    if "." not in code:
        return 0
    fraction = code.split(".", 1)[1]
    if any(character in "#?" for character in fraction):
        return None
    return fraction.count("0")


def _split_literals(code: str) -> tuple[str, str, str]:
    """Splits a section into leading literal, trailing literal and body.

    Currency markers such as `[$$-409]` and quoted runs become literal text;
    alignment and fill directives contribute no visible glyph.
    """
    prefix: list[str] = []
    suffix: list[str] = []
    body: list[str] = []

    def push(value: str) -> None:
        target = suffix if any(item in _PLACEHOLDERS for item in body) else prefix
        target.append(value)

    index = 0
    length = len(code)
    while index < length:
        character = code[index]
        if character == '"':
            index += 1
            start = index
            while index < length and code[index] != '"':
                index += 1
            push(code[start:index])
        elif character == "[":
            index += 1
            start = index
            while index < length and code[index] != "]":
                index += 1
            token = code[start:index]
            # `[$SYMBOL-LOCALE]` carries a currency symbol; colour and
            # condition tokens carry none.
            if token.startswith("$"):
                push(token[1:].split("-", 1)[0])
        elif character == "\\":
            index += 1
            if index < length:
                push(code[index])
        elif character in {"_", "*"}:
            index += 1
        elif character in _CURRENCY_CHARS:
            push(character)
        else:
            body.append(character)
        index += 1
    return ("".join(prefix), "".join(suffix), "".join(body))


def render_decimal(value: float, spec: DecimalFormat) -> str:
    """Renders a fixed-point decimal, matching the Rust implementation."""
    negative = value < 0
    rendered = f"{abs(value):.{spec.decimals}f}"
    integer, _, fraction = rendered.partition(".")
    digits = _group_thousands(integer) if spec.grouped else integer
    if fraction:
        digits = f"{digits}.{fraction}"
    body = f"{spec.prefix}{digits}{spec.suffix}"
    if not negative:
        return body
    return f"({body})" if spec.parenthesized_negative else f"-{body}"


def _group_thousands(digits: str) -> str:
    count = len(digits)
    output: list[str] = []
    for index, character in enumerate(digits):
        if index > 0 and (count - index) % 3 == 0:
            output.append(",")
        output.append(character)
    return "".join(output)
