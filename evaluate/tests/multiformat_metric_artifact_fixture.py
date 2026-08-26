from __future__ import annotations

import hashlib
import json
import struct
import zlib
from pathlib import Path

from evaluate.multiformat_schema import JsonValue


def write_unit_artifacts(
    evidence_root: Path,
    unit_id: str,
    width: int,
    height: int,
) -> dict[str, JsonValue]:
    artifact_root = evidence_root / "artifacts"
    artifact_root.mkdir(exist_ok=True)
    bindings: dict[str, JsonValue] = {}
    for role in ["reference", "candidate"]:
        png = artifact_root / f"{unit_id}-{role}.png"
        inventory = artifact_root / f"{unit_id}-{role}.json"
        write_png(png, width, height, (40, 80, 120))
        inventory.write_text(
            json.dumps(
                {
                    "schema_version": 1,
                    "unit_id": unit_id,
                    "texts": [],
                    "cells": [],
                    "objects": [],
                    "unattributed_cells": [],
                },
                sort_keys=True,
            ),
            encoding="utf-8",
        )
        bindings[f"{role}_png"] = binding(evidence_root, png)
        bindings[f"{role}_inventory"] = binding(evidence_root, inventory)
    return bindings


def write_png(
    path: Path,
    width: int,
    height: int,
    color: tuple[int, int, int],
) -> None:
    row = b"\x00" + bytes(color) * width
    raw = row * height

    def chunk(kind: bytes, data: bytes) -> bytes:
        body = kind + data
        return (
            struct.pack(">I", len(data))
            + body
            + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)
        )

    path.write_bytes(
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(raw))
        + chunk(b"IEND", b"")
    )


def write_checkerboard_png(path: Path, width: int, height: int) -> None:
    rows = []
    for row_index in range(height):
        row = bytearray(b"\x00")
        for column_index in range(width):
            value = 255 if (row_index + column_index) % 2 else 0
            row.extend((value, value, value))
        rows.append(bytes(row))
    raw = b"".join(rows)

    def chunk(kind: bytes, data: bytes) -> bytes:
        body = kind + data
        return (
            struct.pack(">I", len(data))
            + body
            + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)
        )

    path.write_bytes(
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(raw))
        + chunk(b"IEND", b"")
    )


def binding(root: Path, path: Path) -> dict[str, JsonValue]:
    return {
        "path": path.relative_to(root).as_posix(),
        "sha256": sha256(path),
    }


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()
