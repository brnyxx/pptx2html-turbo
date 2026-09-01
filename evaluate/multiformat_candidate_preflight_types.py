from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

from evaluate.multiformat_candidate_runtime_profile import CandidateRuntimeProfile
from evaluate.multiformat_candidate_sandbox import CandidateSandbox
from evaluate.multiformat_candidate_sources import CandidateSourceSet
from evaluate.multiformat_candidate_types import CandidateRuntimePaths


@dataclass(frozen=True, slots=True)
class CandidatePreflight:
    source_set: CandidateSourceSet
    runtime: CandidateRuntimePaths
    project_revision: str
    runtime_tools: dict[str, str]
    runtime_artifacts: dict[str, Path]
    font_bundle_sha256: str
    runtime_profile: CandidateRuntimeProfile
    sandbox: CandidateSandbox | None


__all__ = ["CandidatePreflight"]
