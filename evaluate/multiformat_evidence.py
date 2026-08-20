from __future__ import annotations

import json
from pathlib import Path

from evaluate.multiformat_schema import (
    JsonValue,
    object_value,
    read_object,
    sha256_file,
    sha256_value,
    string_value,
)


class EvidencePathError(Exception):
    pass


def bound_evidence_path(
    report: dict[str, JsonValue],
    field: str,
    evidence_root: Path,
    failures: list[str],
) -> Path | None:
    try:
        binding = object_value(report, field)
        relative_path = string_value(binding, "path")
        expected_hash = sha256_value(binding, "sha256")
        evidence_path = resolve_evidence_path(evidence_root, relative_path)
        if sha256_file(evidence_path) != expected_hash:
            failures.append(field)
            return None
        return evidence_path
    except (OSError, TypeError, ValueError, EvidencePathError):
        failures.append(field)
        return None


def resolve_evidence_path(root: Path, relative_path: str) -> Path:
    relative = Path(relative_path)
    if (
        relative.is_absolute()
        or "\\" in relative_path
        or relative_path != relative.as_posix()
        or any(part in {"", ".", ".."} for part in relative.parts)
    ):
        raise EvidencePathError("evidence path must be normalized and relative")
    resolved_root = root.resolve(strict=True)
    candidate = resolved_root
    for part in relative.parts:
        candidate /= part
        if candidate.is_symlink():
            raise EvidencePathError("evidence path cannot contain symlinks")
    candidate = candidate.resolve(strict=True)
    if not candidate.is_relative_to(resolved_root) or not candidate.is_file():
        raise EvidencePathError("evidence path escapes the evidence root")
    return candidate


def oracle_lock_ready(path: Path) -> bool:
    if not path.is_file():
        return False
    try:
        lock = read_object(path)
        if lock.get("schema_version") != 1 or lock.get("status") != "locked":
            return False
        office = object_value(lock, "office")
        pdf = object_value(lock, "pdf")
        browser = object_value(lock, "browser")
        for field in ["os", "word", "excel", "powerpoint"]:
            string_value(office, field)
        for field in ["primary", "secondary"]:
            string_value(pdf, field)
        string_value(browser, "chromium")
        font_hash = string_value(lock, "font_bundle_sha256")
        return len(font_hash) == 64 and all(
            character in "0123456789abcdef" for character in font_hash
        )
    except (OSError, UnicodeError, json.JSONDecodeError, ValueError, TypeError):
        return False
