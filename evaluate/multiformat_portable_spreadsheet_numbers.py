"""Classification of the bounded spreadsheet number-format contract."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Final

GENERAL: Final = "general"
DECIMAL: Final = "decimal"
PERCENT: Final = "percent"
ISO_DATE: Final = "iso_date"
ISO_DATE_TIME: Final = "iso_date_time"
SCIENTIFIC: Final = "scientific"
TIME: Final = "time"
UNSUPPORTED: Final = "unsupported"

_PLACEHOLDERS: Final = frozenset("0#?")
_NUMERIC_BODY: Final = frozenset("0#.,-+ ?")
_PERCENT_BODY: Final = frozenset("0#., ?")
_CURRENCY_CHARS: Final = frozenset("$\u00a4\u20ac\u00a3\u00a5\u20a9")
_MAX_FORMAT_CODE_CHARS: Final = 254
_MAX_RENDERED_DIGITS: Final = 64


@dataclass(frozen=True, slots=True)
class DecimalFormat:
    minimum_decimals: int = 0
    maximum_decimals: int = 0
    grouped: bool = False
    prefix: str = ""
    suffix: str = ""
    parenthesized_negative: bool = False


@dataclass(frozen=True, slots=True)
class ResolvedFormat:
    kind: str
    decimals: int = 0
    exponent_digits: int = 0
    uppercase: bool = True
    show_positive_sign: bool = True
    padded_hour: bool = False
    spec: DecimalFormat = field(default_factory=DecimalFormat)


GENERAL_FORMAT: Final = ResolvedFormat(GENERAL)
UNSUPPORTED_FORMAT: Final = ResolvedFormat(UNSUPPORTED)


def _plain(decimals: int, grouped: bool) -> ResolvedFormat:
    return ResolvedFormat(
        DECIMAL,
        spec=DecimalFormat(
            minimum_decimals=decimals,
            maximum_decimals=decimals,
            grouped=grouped,
        ),
    )


def _accounting(decimals: int) -> ResolvedFormat:
    return ResolvedFormat(
        DECIMAL,
        spec=DecimalFormat(
            minimum_decimals=decimals,
            maximum_decimals=decimals,
            grouped=True,
            parenthesized_negative=True,
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
    if len(code) > _MAX_FORMAT_CODE_CHARS:
        return UNSUPPORTED_FORMAT
    if code.strip().lower() == "general":
        return GENERAL_FORMAT
    sections = code.split(";")
    positive = sections[0]
    if len(sections) > 2:
        return UNSUPPORTED_FORMAT
    negative = sections[1] if len(sections) == 2 else None
    prefix, suffix, body, conditional = _split_literals(positive)
    if conditional:
        return UNSUPPORTED_FORMAT
    body = body.strip()
    if not body:
        return UNSUPPORTED_FORMAT
    temporal = _temporal_format(body, prefix, suffix)
    if temporal is not None:
        if negative is None or negative.strip() == "@":
            return temporal
        return UNSUPPORTED_FORMAT
    if _has_scaling_comma(body):
        return UNSUPPORTED_FORMAT
    parenthesized_negative = False
    if negative is not None:
        trimmed = negative.strip()
        # Parentheses are the accounting convention; any other rewrite of the
        # negative form is not reproduced.
        if not (
            (trimmed.startswith("(") and trimmed.endswith(")"))
            or (trimmed.startswith("\\(") and trimmed.endswith("\\)"))
        ):
            return UNSUPPORTED_FORMAT
        parenthesized_negative = True
    if "%" in body:
        return _percent_format(body, parenthesized_negative)
    if "E" in body or "e" in body:
        return _scientific_format(body, prefix, suffix, parenthesized_negative)
    return _numeric_format(body, prefix, suffix, parenthesized_negative)


def _temporal_format(body: str, prefix: str, suffix: str) -> ResolvedFormat | None:
    if not all(
        character.isspace() or character in "-/.:," for character in prefix + suffix
    ):
        return None
    lowered = body.lower()
    has_date = "y" in lowered or "d" in lowered
    has_time = "h" in lowered or "s" in lowered
    if has_date:
        return ResolvedFormat(ISO_DATE_TIME if has_time else ISO_DATE)
    if has_time:
        compact = "".join(character for character in lowered if not character.isspace())
        if compact == "h:mm:ss":
            return ResolvedFormat(TIME)
        if compact == "hh:mm:ss":
            return ResolvedFormat(TIME, padded_hour=True)
        return None
    return ResolvedFormat(ISO_DATE) if "m" in lowered else None


def _percent_format(body: str, parenthesized_negative: bool) -> ResolvedFormat:
    if parenthesized_negative:
        return UNSUPPORTED_FORMAT
    numeric = body.replace("%", "")
    if not all(character in _PERCENT_BODY for character in numeric):
        return UNSUPPORTED_FORMAT
    places = _decimal_places(numeric)
    if places is None or places[0] != places[1]:
        return UNSUPPORTED_FORMAT
    return ResolvedFormat(PERCENT, decimals=places[0])


def _scientific_format(
    body: str,
    prefix: str,
    suffix: str,
    parenthesized_negative: bool,
) -> ResolvedFormat:
    if parenthesized_negative or prefix or suffix:
        return UNSUPPORTED_FORMAT
    markers = [(index, body[index]) for index in range(len(body)) if body[index] in "Ee"]
    if len(markers) != 1:
        return UNSUPPORTED_FORMAT
    index, marker = markers[0]
    mantissa = body[:index]
    exponent = body[index + 1 :]
    if not exponent or exponent[0] not in "+-":
        return UNSUPPORTED_FORMAT
    exponent_digits = exponent[1:]
    places = _decimal_places(mantissa)
    if (
        not exponent_digits
        or len(exponent_digits) > _MAX_RENDERED_DIGITS
        or any(character != "0" for character in exponent_digits)
        or places is None
        or places[0] != places[1]
        or any(character not in "0#." for character in mantissa)
        or mantissa.split(".", 1)[0] != "0"
    ):
        return UNSUPPORTED_FORMAT
    return ResolvedFormat(
        SCIENTIFIC,
        decimals=places[0],
        exponent_digits=len(exponent_digits),
        uppercase=marker == "E",
        show_positive_sign=exponent[0] == "+",
    )


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
    places = _decimal_places(body)
    if places is None:
        return UNSUPPORTED_FORMAT
    return ResolvedFormat(
        DECIMAL,
        spec=DecimalFormat(
            minimum_decimals=places[0],
            maximum_decimals=places[1],
            grouped="," in integer,
            prefix=prefix,
            suffix=suffix,
            parenthesized_negative=parenthesized_negative,
        ),
    )


def _decimal_places(code: str) -> tuple[int, int] | None:
    if "." not in code:
        return (0, 0)
    fraction = code.split(".", 1)[1]
    if "?" in fraction:
        return None
    minimum = fraction.count("0")
    maximum = sum(character in "0#" for character in fraction)
    return (minimum, maximum) if maximum <= _MAX_RENDERED_DIGITS else None


def _has_scaling_comma(code: str) -> bool:
    last_placeholder = max(
        (index for index, character in enumerate(code) if character in _PLACEHOLDERS),
        default=-1,
    )
    return last_placeholder >= 0 and "," in code[last_placeholder + 1 :]


def _split_literals(code: str) -> tuple[str, str, str, bool]:
    """Splits a section into leading literal, trailing literal and body.

    Currency markers such as `[$$-409]` and quoted runs become literal text;
    alignment and fill directives contribute no visible glyph.
    """
    prefix: list[str] = []
    suffix: list[str] = []
    body: list[str] = []
    conditional = False

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
            conditional = conditional or token.lstrip().startswith(("<", ">", "="))
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
    return ("".join(prefix), "".join(suffix), "".join(body), conditional)


def render_decimal(value: float, spec: DecimalFormat) -> str:
    """Renders a fixed-point decimal, matching the Rust implementation."""
    negative = value < 0
    rendered = f"{abs(value):.{spec.maximum_decimals}f}"
    integer, _, fraction = rendered.partition(".")
    digits = _group_thousands(integer) if spec.grouped else integer
    while len(fraction) > spec.minimum_decimals and fraction.endswith("0"):
        fraction = fraction[:-1]
    if fraction:
        digits = f"{digits}.{fraction}"
    body = f"{spec.prefix}{digits}{spec.suffix}"
    if not negative:
        return body
    return f"({body})" if spec.parenthesized_negative else f"-{body}"


def render_scientific(value: float, resolved: ResolvedFormat) -> str:
    rendered = f"{abs(value):.{resolved.decimals}e}"
    mantissa, exponent_text = rendered.split("e", 1)
    exponent = int(exponent_text)
    marker = "E" if resolved.uppercase else "e"
    exponent_sign = "-" if exponent < 0 else "+" if resolved.show_positive_sign else ""
    value_sign = "-" if value < 0 else ""
    digits = f"{abs(exponent):0{resolved.exponent_digits}d}"
    return f"{value_sign}{mantissa}{marker}{exponent_sign}{digits}"


def _group_thousands(digits: str) -> str:
    count = len(digits)
    output: list[str] = []
    for index, character in enumerate(digits):
        if index > 0 and (count - index) % 3 == 0:
            output.append(",")
        output.append(character)
    return "".join(output)
