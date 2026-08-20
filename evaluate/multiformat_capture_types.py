from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True, slots=True)
class ArtifactIdentity:
    path: str
    sha256: str


@dataclass(frozen=True, slots=True)
class CaptureUnit:
    unit_id: str
    source_id: str
    source_sha256: str
    ordinal: int
    png: ArtifactIdentity
    inventory: ArtifactIdentity


@dataclass(frozen=True, slots=True)
class CaptureFile:
    source_id: str
    source_sha256: str
    html: ArtifactIdentity


@dataclass(frozen=True, slots=True)
class CaptureManifest:
    units: dict[str, CaptureUnit]
    files: dict[str, CaptureFile]
    determinism_path: Path | None
