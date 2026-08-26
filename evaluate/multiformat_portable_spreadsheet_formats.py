"""Number-format resolution for portable XLSX display text.

Mirrors `crates/document2html-core/src/spreadsheet/styles.rs` and
`display.rs`. Only a bounded subset of ECMA-376 format codes is reproduced;
anything else yields `UNATTRIBUTABLE` so the cell still converts but is
excluded from coordinate attribution instead of publishing a value that
differs from what a spreadsheet application shows.
"""

from __future__ import annotations

from typing import Final
from xml.etree import ElementTree

MAIN: Final = "http://schemas.openxmlformats.org/spreadsheetml/2006/main"

# Sentinel for a value whose visible text cannot be reproduced.
UNATTRIBUTABLE: Final = object()

GENERAL: Final = "general"
PERCENT: Final = "percent"
ISO_DATE: Final = "iso_date"
ISO_DATE_TIME: Final = "iso_date_time"
UNSUPPORTED: Final = "unsupported"

# Built-in numFmt ids reproduced here, matching the Rust core exactly.
# 0 General, 1 `0`, 2 `0.00`, 3/4 thousands-separated, 49 Text.
_BUILTIN: Final[dict[str, tuple[str, int]]] = {
    "0": (GENERAL, 0),
    "1": (GENERAL, 0),
    "2": (GENERAL, 0),
    "3": (GENERAL, 0),
    "4": (GENERAL, 0),
    "49": (GENERAL, 0),
    "9": (PERCENT, 0),
    "10": (PERCENT, 2),
    "14": (ISO_DATE, 0),
    "15": (ISO_DATE, 0),
    "16": (ISO_DATE, 0),
    "17": (ISO_DATE, 0),
    "22": (ISO_DATE_TIME, 0),
}

# Days from the 1900 serial epoch to 1970-01-01, accounting for the
# spreadsheet's non-existent 1900-02-29 leap day.
_SERIAL_EPOCH_OFFSET: Final = 25_569
_SECONDS_PER_DAY: Final = 86_400
_NUMERIC_PLACEHOLDERS: Final = frozenset("0#.,-+ ?")
_PERCENT_PLACEHOLDERS: Final = frozenset("0#., ?")


def parse_styles(root: ElementTree.Element) -> list[tuple[str, int]]:
    """Builds the cell-format table from a parsed `xl/styles.xml` root."""
    custom = {
        item.attrib["numFmtId"]: item.attrib["formatCode"]
        for item in root.iter(f"{{{MAIN}}}numFmt")
        if "numFmtId" in item.attrib and "formatCode" in item.attrib
    }
    formats: list[tuple[str, int]] = []
    for container in root.iter(f"{{{MAIN}}}cellXfs"):
        for item in container.findall(f"{{{MAIN}}}xf"):
            identity = item.attrib.get("numFmtId", "0")
            formats.append(_resolve(identity, custom))
    return formats


def cell_format(
    formats: list[tuple[str, int]], style_index: str | None
) -> tuple[str, int]:
    """Resolves a cell's `s` attribute against the style table."""
    if style_index is None:
        return (GENERAL, 0)
    try:
        index = int(style_index)
    except ValueError:
        return (UNSUPPORTED, 0)
    if 0 <= index < len(formats):
        return formats[index]
    # A missing entry means no format was applied.
    return (GENERAL, 0)


def _resolve(identity: str, custom: dict[str, str]) -> tuple[str, int]:
    code = custom.get(identity)
    if code is not None:
        return _classify(code)
    return _BUILTIN.get(identity, (UNSUPPORTED, 0))


def _classify(code: str) -> tuple[str, int]:
    section = code.split(";", 1)[0]
    stripped = _strip_literals(section)
    if not stripped:
        return (UNSUPPORTED, 0)
    if "%" in stripped:
        return _percent_format(stripped)
    lowered = stripped.lower()
    if any(marker in lowered for marker in "ydm"):
        if any(marker in lowered for marker in "hs"):
            return (ISO_DATE_TIME, 0)
        return (ISO_DATE, 0)
    if all(character in _NUMERIC_PLACEHOLDERS for character in stripped):
        return (GENERAL, 0)
    return (UNSUPPORTED, 0)


def _strip_literals(code: str) -> str:
    output: list[str] = []
    index = 0
    length = len(code)
    while index < length:
        character = code[index]
        if character == '"':
            index += 1
            while index < length and code[index] != '"':
                index += 1
        elif character == "[":
            while index < length and code[index] != "]":
                index += 1
        elif character in {"\\", "_"}:
            index += 1
        elif character == "*":
            pass
        else:
            output.append(character)
        index += 1
    return "".join(output)


def _percent_format(code: str) -> tuple[str, int]:
    numeric = code.replace("%", "")
    decimals = 0
    if "." in numeric:
        decimals = numeric.split(".", 1)[1].count("0")
    if all(character in _PERCENT_PLACEHOLDERS for character in numeric):
        return (PERCENT, decimals)
    return (UNSUPPORTED, 0)


def formatted_value(raw: str, resolved: tuple[str, int]) -> str | object:
    """Renders `raw` under `resolved`, or `UNATTRIBUTABLE`."""
    kind, decimals = resolved
    if kind == GENERAL:
        return raw
    if kind == PERCENT:
        try:
            value = float(raw)
        except ValueError:
            return UNATTRIBUTABLE
        return f"{value * 100:.{decimals}f}%"
    if kind in {ISO_DATE, ISO_DATE_TIME}:
        return _serial_to_iso(raw, kind == ISO_DATE_TIME)
    return UNATTRIBUTABLE


def _serial_to_iso(raw: str, with_time: bool) -> str | object:
    try:
        serial = float(raw)
    except ValueError:
        return UNATTRIBUTABLE
    if serial < 1.0:
        return UNATTRIBUTABLE
    days = int(serial)
    fraction = serial - days
    seconds = min(max(round(fraction * _SECONDS_PER_DAY), 0), _SECONDS_PER_DAY)
    year, month, day = _civil_from_days(days - _SERIAL_EPOCH_OFFSET)
    if not with_time:
        return f"{year:04d}-{month:02d}-{day:02d}"
    hour, minute, second = seconds // 3600, (seconds % 3600) // 60, seconds % 60
    return f"{year:04d}-{month:02d}-{day:02d}T{hour:02d}:{minute:02d}:{second:02d}"


def _civil_from_days(days: int) -> tuple[int, int, int]:
    """Howard Hinnant's civil-from-days, matching the Rust implementation."""
    shifted = days + 719_468
    era = (shifted if shifted >= 0 else shifted - 146_096) // 146_097
    day_of_era = shifted - era * 146_097
    year_of_era = (
        day_of_era - day_of_era // 1460 + day_of_era // 36_524 - day_of_era // 146_096
    ) // 365
    year = year_of_era + era * 400
    day_of_year = day_of_era - (
        365 * year_of_era + year_of_era // 4 - year_of_era // 100
    )
    shifted_month = (5 * day_of_year + 2) // 153
    day = day_of_year - (153 * shifted_month + 2) // 5 + 1
    month = shifted_month + 3 if shifted_month < 10 else shifted_month - 9
    return (year + (1 if month <= 2 else 0), month, day)


def iso_date_text(raw: str) -> str | object:
    """Normalizes an ISO date stored as text by a `t="d"` cell."""
    trimmed = raw.strip()
    date, _, time = trimmed.partition("T")
    if not _valid_iso_date(date):
        return UNATTRIBUTABLE
    if not time:
        return date
    if time.rstrip("Z") in {"00:00", "00:00:00", "00:00:00.000"}:
        return date
    if _valid_iso_time(time):
        return f"{date}T{time}"
    return UNATTRIBUTABLE


def _valid_iso_date(value: str) -> bool:
    parts = value.split("-")
    if len(parts) != 3 or [len(item) for item in parts] != [4, 2, 2]:
        return False
    if not all(item.isdigit() for item in parts):
        return False
    year, month, day = (int(item) for item in parts)
    return 1 <= month <= 12 and 1 <= day <= _days_in_month(year, month)


def _days_in_month(year: int, month: int) -> int:
    if month in {1, 3, 5, 7, 8, 10, 12}:
        return 31
    if month in {4, 6, 9, 11}:
        return 30
    if month == 2:
        leap = year % 4 == 0 and (year % 100 != 0 or year % 400 == 0)
        return 29 if leap else 28
    return 0


def _valid_iso_time(value: str) -> bool:
    parts = value.rstrip("Z").split(":")
    if len(parts) != 3:
        return False
    second = parts[2].split(".", 1)[0]
    if not all(item.isdigit() for item in (parts[0], parts[1], second)):
        return False
    return int(parts[0]) < 24 and int(parts[1]) < 60 and int(second) < 60
