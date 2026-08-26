from __future__ import annotations

from pathlib import Path


class EvidencePathError(Exception):
    pass


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
