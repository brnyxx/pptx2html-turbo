from __future__ import annotations

import binascii
import hashlib
import json
import re
import struct
import zlib
from collections.abc import Mapping
from datetime import datetime
from pathlib import Path

MAX_JSON_BYTES = 1_048_576
MAX_PNG_BYTES = 64 * 1024 * 1024
MAX_PNG_DIMENSION = 16_384
PRODUCER = "Microsoft PowerPoint"
PLATFORM = "Windows"
REQUIRED_PROVENANCE_FIELDS = (
    "producer",
    "platform",
    "powerpoint_version",
    "powerpoint_build",
    "capture_timestamp",
    "batch_id",
    "powerpoint_channel",
    "windows_version",
    "export_command",
    "output_resolution",
    "golden_set_revision",
)
SHA256_RE = re.compile(r"[0-9a-f]{64}")
BATCH_ID_RE = re.compile(r"[A-Za-z0-9][A-Za-z0-9._-]{7,127}")


class ProvenanceError(RuntimeError):
    pass


def sha256_file(path: Path, *, max_bytes: int | None = None) -> str:
    size = path.stat().st_size
    if max_bytes is not None and size > max_bytes:
        raise ProvenanceError(f"FILE_TOO_LARGE:{path}")
    digest = hashlib.sha256()
    with path.open("rb") as source:
        while chunk := source.read(1024 * 1024):
            digest.update(chunk)
    return digest.hexdigest()


def read_json(path: Path) -> dict[str, object]:
    if not path.is_file() or path.stat().st_size > MAX_JSON_BYTES:
        raise ProvenanceError(f"JSON_FILE_INVALID:{path}")
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ProvenanceError(f"JSON_FILE_INVALID:{path}") from error
    if not isinstance(payload, dict):
        raise ProvenanceError(f"JSON_ROOT_INVALID:{path}")
    return payload


def validate_provenance(payload: Mapping[str, object]) -> list[str]:
    errors = [
        f"PROVENANCE_FIELD_INVALID:{field}"
        for field in REQUIRED_PROVENANCE_FIELDS
        if not isinstance(payload.get(field), str) or not str(payload[field]).strip()
    ]
    if payload.get("producer") != PRODUCER:
        errors.append("PROVENANCE_PRODUCER_NOT_POWERPOINT")
    if payload.get("platform") != PLATFORM or not str(
        payload.get("windows_version", "")
    ).startswith("Windows"):
        errors.append("PROVENANCE_PLATFORM_NOT_WINDOWS")
    if not re.fullmatch(r"[0-9]+(?:\.[0-9A-Za-z-]+)+", str(payload.get("powerpoint_version", ""))):
        errors.append("PROVENANCE_POWERPOINT_VERSION_INVALID")
    if not re.fullmatch(r"[0-9]+(?:\.[0-9]+)+", str(payload.get("powerpoint_build", ""))):
        errors.append("PROVENANCE_POWERPOINT_BUILD_INVALID")
    try:
        timestamp = datetime.fromisoformat(
            str(payload.get("capture_timestamp", "")).replace("Z", "+00:00")
        )
        if timestamp.tzinfo is None:
            errors.append("PROVENANCE_CAPTURE_TIMESTAMP_INVALID")
    except ValueError:
        errors.append("PROVENANCE_CAPTURE_TIMESTAMP_INVALID")
    if BATCH_ID_RE.fullmatch(str(payload.get("batch_id", ""))) is None:
        errors.append("PROVENANCE_BATCH_ID_INVALID")
    export_command = str(payload.get("export_command", ""))
    if re.search(
        r"(?:^|[\s\\/])reference_render_powerpoint\.ps1(?:\s|$)",
        export_command,
        re.IGNORECASE,
    ) is None:
        errors.append("PROVENANCE_EXPORT_COMMAND_INVALID")
    return errors


def validate_png(path: Path) -> dict[str, object]:
    if not path.is_file() or not 0 < path.stat().st_size <= MAX_PNG_BYTES:
        raise ProvenanceError(f"PNG_FILE_INVALID:{path}")
    data = path.read_bytes()
    if not data.startswith(b"\x89PNG\r\n\x1a\n"):
        raise ProvenanceError(f"PNG_SIGNATURE_INVALID:{path}")
    offset = 8
    chunks: list[bytes] = []
    image_data = bytearray()
    width = height = 0
    while offset < len(data):
        if offset + 12 > len(data):
            raise ProvenanceError(f"PNG_CHUNK_TRUNCATED:{path}")
        length = struct.unpack(">I", data[offset : offset + 4])[0]
        kind = data[offset + 4 : offset + 8]
        end = offset + 12 + length
        if length > MAX_PNG_BYTES or end > len(data):
            raise ProvenanceError(f"PNG_CHUNK_BOUNDS_INVALID:{path}")
        payload = data[offset + 8 : offset + 8 + length]
        expected_crc = struct.unpack(">I", data[offset + 8 + length : end])[0]
        if binascii.crc32(kind + payload) & 0xFFFFFFFF != expected_crc:
            raise ProvenanceError(f"PNG_CRC_INVALID:{path}")
        chunks.append(kind)
        if len(chunks) == 1:
            if kind != b"IHDR" or length != 13:
                raise ProvenanceError(f"PNG_IHDR_INVALID:{path}")
            width, height, bit_depth, color_type, compression, filter_method, interlace = (
                struct.unpack(">IIBBBBB", payload)
            )
            if not (0 < width <= MAX_PNG_DIMENSION and 0 < height <= MAX_PNG_DIMENSION):
                raise ProvenanceError(f"PNG_DIMENSIONS_INVALID:{path}")
            if (
                bit_depth != 8
                or color_type != 6
                or compression != 0
                or filter_method != 0
                or interlace != 0
            ):
                raise ProvenanceError(f"PNG_IHDR_FORMAT_INVALID:{path}")
        elif kind == b"IDAT":
            image_data.extend(payload)
        if kind == b"IEND":
            if length != 0 or end != len(data):
                raise ProvenanceError(f"PNG_IEND_INVALID:{path}")
            break
        offset = end
    if not chunks or chunks[-1] != b"IEND" or b"IDAT" not in chunks:
        raise ProvenanceError(f"PNG_STRUCTURE_INVALID:{path}")
    try:
        scanlines = zlib.decompress(bytes(image_data))
    except zlib.error as error:
        raise ProvenanceError(f"PNG_IDAT_INVALID:{path}") from error
    expected_scanline_bytes = height * (1 + width * 4)
    if len(scanlines) != expected_scanline_bytes:
        raise ProvenanceError(f"PNG_SCANLINE_SIZE_INVALID:{path}")
    row_bytes = 1 + width * 4
    if any(scanlines[offset] > 4 for offset in range(0, len(scanlines), row_bytes)):
        raise ProvenanceError(f"PNG_FILTER_INVALID:{path}")
    return {
        "file": path.name,
        "sha256": hashlib.sha256(data).hexdigest(),
        "width": width,
        "height": height,
        "bytes": len(data),
    }


def canonical_json_sha256(payload: object) -> str:
    encoded = json.dumps(payload, sort_keys=True, separators=(",", ":")).encode()
    return hashlib.sha256(encoded).hexdigest()


def matching_provenance(left: Mapping[str, object], right: Mapping[str, object]) -> bool:
    return all(left.get(field) == right.get(field) for field in REQUIRED_PROVENANCE_FIELDS)
