from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path


class CandidateCaptureError(Exception):
    pass


@dataclass(frozen=True, slots=True)
class CapturedUnit:
    unit_id: str
    png: Path
    inventory: Path


@dataclass(frozen=True, slots=True)
class BrowserCaptureResult:
    browser_version: str
    units: tuple[CapturedUnit, ...]
    external_requests: tuple[str, ...]


@dataclass(frozen=True, slots=True)
class CapturedSource:
    track: str
    source_id: str
    source_sha256: str
    html: Path
    inventory_manifest: Path
    units: tuple[CapturedUnit, ...]


@dataclass(frozen=True, slots=True)
class CandidateRun:
    run_id: int
    browser_version: str
    sources: tuple[CapturedSource, ...]


@dataclass(frozen=True, slots=True)
class CandidateManifestPaths:
    capture: Path
    upstream: Path
    execution: Path
    runtime_identity: Path
    determinism: Path


@dataclass(frozen=True, slots=True)
class CandidateRuntimePaths:
    converter: Path
    soffice: Path
    pdftohtml: Path
    pdfinfo: Path
    chromium: Path
    receipt_signer: Path
    font_config: Path
    browser_version: str
    timeout_seconds: int
