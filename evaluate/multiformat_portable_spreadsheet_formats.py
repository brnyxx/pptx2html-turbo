"""Number-format resolution for portable XLSX display text.

Mirrors `crates/document2html-core/src/spreadsheet/styles.rs` and
`display.rs`. Format-code classification lives in
`multiformat_portable_spreadsheet_numbers`.

An unreproducible format yields `UNATTRIBUTABLE` so the cell still converts
but is excluded from coordinate attribution instead of publishing a value that
differs from what a spreadsheet application shows.
"""

from __future__ import annotations

import math
from typing import Final
from xml.etree import ElementTree

from evaluate.multiformat_portable_spreadsheet_numbers import (
    DECIMAL,
    GENERAL,
    ISO_DATE,
    ISO_DATE_TIME,
    PERCENT,
    SCIENTIFIC,
    TIME,
    UNSUPPORTED,
    ResolvedFormat,
    builtin_format,
    classify_format_code,
    render_decimal,
    render_scientific,
)

MAIN: Final = "http://schemas.openxmlformats.org/spreadsheetml/2006/main"

# Sentinel for a value whose visible text cannot be reproduced.
UNATTRIBUTABLE: Final = object()

# Days from the 1900 serial epoch to 1970-01-01, accounting for the
# spreadsheet's non-existent 1900-02-29 leap day.
_SERIAL_EPOCH_OFFSET: Final = 25_569
_SECONDS_PER_DAY: Final = 86_400


def parse_styles(root: ElementTree.Element) -> list[ResolvedFormat]:
    """Builds the cell-format table from a parsed `xl/styles.xml` root."""
    custom = {
        item.attrib["numFmtId"]: item.attrib["formatCode"]
        for item in root.iter(f"{{{MAIN}}}numFmt")
        if "numFmtId" in item.attrib and "formatCode" in item.attrib
    }
    formats: list[ResolvedFormat] = []
    for container in root.iter(f"{{{MAIN}}}cellXfs"):
        for item in container.findall(f"{{{MAIN}}}xf"):
            identity = item.attrib.get("numFmtId", "0")
            code = custom.get(identity)
            formats.append(
                classify_format_code(code)
                if code is not None
                else builtin_format(identity)
            )
    return formats


def cell_format(
    formats: list[ResolvedFormat], style_index: str | None
) -> ResolvedFormat:
    """Resolves a cell's `s` attribute against the style table."""
    if style_index is None:
        return ResolvedFormat(GENERAL)
    try:
        index = int(style_index)
    except ValueError:
        return ResolvedFormat(UNSUPPORTED)
    if 0 <= index < len(formats):
        return formats[index]
    # A missing entry means no format was applied.
    return ResolvedFormat(GENERAL)


def formatted_value(raw: str, resolved: ResolvedFormat) -> str | object:
    """Renders `raw` under `resolved`, or `UNATTRIBUTABLE`."""
    if resolved.kind == GENERAL:
        return raw
    if resolved.kind == DECIMAL:
        try:
            value = float(raw)
        except ValueError:
            return UNATTRIBUTABLE
        return render_decimal(value, resolved.spec)
    if resolved.kind == PERCENT:
        try:
            value = float(raw)
        except ValueError:
            return UNATTRIBUTABLE
        return f"{value * 100:.{resolved.decimals}f}%"
    if resolved.kind == SCIENTIFIC:
        try:
            value = float(raw)
        except ValueError:
            return UNATTRIBUTABLE
        if not math.isfinite(value):
            return UNATTRIBUTABLE
        return render_scientific(value, resolved)
    if resolved.kind in {ISO_DATE, ISO_DATE_TIME}:
        return _serial_to_iso(raw, resolved.kind == ISO_DATE_TIME)
    if resolved.kind == TIME:
        return _serial_to_time(raw, resolved.padded_hour)
    return UNATTRIBUTABLE


def _serial_to_time(raw: str, padded_hour: bool) -> str | object:
    try:
        serial = float(raw)
    except ValueError:
        return UNATTRIBUTABLE
    if not math.isfinite(serial) or serial < 0:
        return UNATTRIBUTABLE
    seconds = math.floor((serial % 1) * _SECONDS_PER_DAY + 0.5) % _SECONDS_PER_DAY
    hour, remainder = divmod(seconds, 3600)
    minute, second = divmod(remainder, 60)
    hour_text = f"{hour:02d}" if padded_hour else str(hour)
    return f"{hour_text}:{minute:02d}:{second:02d}"


def _serial_to_iso(raw: str, with_time: bool) -> str | object:
    try:
        serial = float(raw)
    except ValueError:
        return UNATTRIBUTABLE
    if serial < 1.0:
        return UNATTRIBUTABLE
    days = int(serial)
    fraction = serial - days
    seconds = round(fraction * _SECONDS_PER_DAY)
    # Rounding up from 23:59:59.5 carries into the next day rather than
    # emitting an out-of-range 24:00:00.
    if seconds >= _SECONDS_PER_DAY:
        seconds -= _SECONDS_PER_DAY
        days += 1
    seconds = max(seconds, 0)
    unix_days = _unix_days_from_serial(days)
    if unix_days is None:
        return UNATTRIBUTABLE
    year, month, day = _civil_from_days(unix_days)
    if not with_time:
        return f"{year:04d}-{month:02d}-{day:02d}"
    hour, minute, second = seconds // 3600, (seconds % 3600) // 60, seconds % 60
    return f"{year:04d}-{month:02d}-{day:02d}T{hour:02d}:{minute:02d}:{second:02d}"


def _unix_days_from_serial(days: int) -> int | None:
    """Converts a 1900-system serial day number to days since 1970-01-01.

    The 1900 system contains a phantom 1900-02-29 at serial 60 that never
    existed. Serials below it are one day ahead of a pure linear mapping, so
    they are shifted; serial 60 itself denotes no real date and is refused.
    """
    if days <= 0 or days == 60:
        return None
    if days <= 59:
        # 1 => 1900-01-01 ... 59 => 1900-02-28
        return days - _SERIAL_EPOCH_OFFSET + 1
    return days - _SERIAL_EPOCH_OFFSET


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
