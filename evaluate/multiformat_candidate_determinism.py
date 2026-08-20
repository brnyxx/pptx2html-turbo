from __future__ import annotations

from pathlib import Path

from evaluate.multiformat_candidate_artifacts import evidence_binding
from evaluate.multiformat_candidate_sources import CandidateSourceSet
from evaluate.multiformat_candidate_types import CandidateCaptureError, CandidateRun
from evaluate.multiformat_schema import JsonValue, sha256_file


class CandidateDeterminismError(CandidateCaptureError):
    pass


def validate_clean_runs(
    source_set: CandidateSourceSet,
    run1: CandidateRun,
    run2: CandidateRun,
) -> None:
    if run1.run_id != 1 or run2.run_id != 2:
        raise CandidateDeterminismError("determinism run IDs must be 1 and 2")
    if run1.browser_version != run2.browser_version:
        raise CandidateDeterminismError("determinism browser versions differ")
    expected = [
        (
            source.track,
            source.source_id,
            source.source_sha256,
            [unit.unit_id for unit in source.units],
        )
        for source in source_set.sources
    ]
    for run in [run1, run2]:
        actual = [
            (
                source.track,
                source.source_id,
                source.source_sha256,
                [unit.unit_id for unit in source.units],
            )
            for source in run.sources
        ]
        if actual != expected:
            raise CandidateDeterminismError("determinism source or unit set differs")
    for left, right in zip(run1.sources, run2.sources, strict=True):
        if sha256_file(left.html) != sha256_file(right.html):
            raise CandidateDeterminismError("determinism HTML differs")
        for left_unit, right_unit in zip(left.units, right.units, strict=True):
            if sha256_file(left_unit.png) != sha256_file(right_unit.png) or sha256_file(
                left_unit.inventory
            ) != sha256_file(right_unit.inventory):
                raise CandidateDeterminismError("determinism unit artifacts differ")


def determinism_run_value(
    root: Path,
    run: CandidateRun,
) -> dict[str, JsonValue]:
    return {
        "run_id": run.run_id,
        "files": [
            {
                "track": source.track,
                "source_id": source.source_id,
                "source_sha256": source.source_sha256,
                "html": evidence_binding(root, source.html),
                "inventory": evidence_binding(root, source.inventory_manifest),
                "png": [evidence_binding(root, unit.png) for unit in source.units],
            }
            for source in run.sources
        ],
    }
