"""Entity-safe parsing of Office oracle PDF layout XML into page geometry."""

from __future__ import annotations

import math
from dataclasses import dataclass
from pathlib import Path
from xml.etree import ElementTree

_POPPLER_XHTML_DOCTYPE = (
    b'<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.0 Transitional//EN" '
    b'"http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd">'
)


class OfficeOracleInventoryError(Exception):
    pass


@dataclass(frozen=True, slots=True)
class LayoutLine:
    value: str
    box: tuple[float, float, float, float]
    baseline: float


@dataclass(frozen=True, slots=True)
class LayoutPage:
    width: float
    height: float
    lines: tuple[LayoutLine, ...]


def layout_pages(path: Path) -> list[LayoutPage]:
    value = path.read_bytes()
    upper = value.upper()
    if b"<!ENTITY" in upper:
        raise OfficeOracleInventoryError("office layout XML is unsafe")
    if b"<!DOCTYPE" in upper:
        if upper.count(b"<!DOCTYPE") != 1 or not value.startswith(
            _POPPLER_XHTML_DOCTYPE
        ):
            raise OfficeOracleInventoryError("office layout XML is unsafe")
        value = value[len(_POPPLER_XHTML_DOCTYPE) :]
    try:
        root = ElementTree.fromstring(value)
    except ElementTree.ParseError as error:
        raise OfficeOracleInventoryError("office layout XML is invalid") from error
    result: list[LayoutPage] = []
    for page in (item for item in root.iter() if _local_name(item.tag) == "page"):
        width = _attribute(page, "width")
        height = _attribute(page, "height")
        if width <= 0 or height <= 0:
            raise OfficeOracleInventoryError("office layout page is invalid")
        lines = tuple(
            parsed
            for item in page.iter()
            if _local_name(item.tag) == "line"
            and (parsed := _layout_line(item)) is not None
        )
        result.append(LayoutPage(width, height, lines))
    if not result:
        raise OfficeOracleInventoryError("office layout has no pages")
    return result


def _layout_line(element: ElementTree.Element) -> LayoutLine | None:
    words = [
        item
        for item in element.iter()
        if _local_name(item.tag) == "word" and (item.text or "").strip()
    ]
    if not words:
        return None
    value = " ".join((item.text or "").strip() for item in words)
    x_min = min(_attribute(item, "xMin") for item in words)
    y_min = min(_attribute(item, "yMin") for item in words)
    x_max = max(_attribute(item, "xMax") for item in words)
    y_max = max(_attribute(item, "yMax") for item in words)
    return LayoutLine(value, (x_min, y_min, x_max - x_min, y_max - y_min), y_max)


def _attribute(element: ElementTree.Element, name: str) -> float:
    try:
        value = float(element.attrib[name])
    except (KeyError, ValueError) as error:
        raise OfficeOracleInventoryError(
            "office layout attribute is invalid"
        ) from error
    if not math.isfinite(value):
        raise OfficeOracleInventoryError("office layout attribute is non-finite")
    return value


def _local_name(value: str) -> str:
    return value.rsplit("}", 1)[-1]


__all__ = [
    "LayoutLine",
    "LayoutPage",
    "OfficeOracleInventoryError",
    "layout_pages",
]
