from __future__ import annotations

import os
import re
from pathlib import Path

from evaluate.multiformat_office_oracle_batch import OfficeOracleBatchUnit
from evaluate.multiformat_reference_routing import DocumentFormat


class PortableReferenceOutputError(ValueError):
    pass


def page_count(path: Path) -> int:
    matches = re.findall(
        r"^Pages:\s+([1-9][0-9]*)\s*$", path.read_text(encoding="utf-8"), re.MULTILINE
    )
    if len(matches) != 1:
        raise PortableReferenceOutputError("pdfinfo page count is invalid")
    return int(matches[0])


def page_images(
    root: Path, count: int, document_format: DocumentFormat
) -> list[OfficeOracleBatchUnit]:
    paths = sorted(
        root.glob("page-*.png"), key=lambda item: int(item.stem.split("-")[-1])
    )
    if len(paths) != count or [int(item.stem.split("-")[-1]) for item in paths] != list(
        range(1, count + 1)
    ):
        raise PortableReferenceOutputError("Poppler page image set differs")
    result = []
    for path in paths:
        width, height = png_dimensions(path)
        if document_format in {DocumentFormat.PPT, DocumentFormat.PPTX} and (
            width,
            height,
        ) != (960, 540):
            raise PortableReferenceOutputError("presentation dimensions differ")
        result.append(OfficeOracleBatchUnit(path, width, height))
    return result


def png_dimensions(path: Path) -> tuple[int, int]:
    value = path.read_bytes()[:24]
    if len(value) != 24 or value[:8] != b"\x89PNG\r\n\x1a\n":
        raise PortableReferenceOutputError("Poppler image is not PNG")
    return int.from_bytes(value[16:20], "big"), int.from_bytes(value[20:24], "big")


def expand(value: str, replacements: dict[str, str]) -> str:
    for name, replacement in replacements.items():
        value = value.replace("{" + name + "}", replacement)
    if "{" in value or "}" in value:
        raise PortableReferenceOutputError("portable route placeholder is unresolved")
    return value


def executable(path: Path) -> str:
    resolved = path.resolve(strict=True)
    if not resolved.is_file() or not os.access(resolved, os.X_OK):
        raise PortableReferenceOutputError("portable reference tool is not executable")
    return resolved.as_posix()
