from __future__ import annotations

from pathlib import Path

from evaluate.multiformat_candidate_artifacts import (
    evidence_binding,
    write_canonical_json,
)
from evaluate.multiformat_candidate_browser import capture_html_units
from evaluate.multiformat_candidate_conversion import run_conversion
from evaluate.multiformat_candidate_sources import CandidateSourceSet
from evaluate.multiformat_candidate_types import (
    CandidateCaptureError,
    CandidateRun,
    CandidateRuntimePaths,
    CapturedSource,
)


class CandidateRunError(CandidateCaptureError):
    pass


def capture_clean_run(
    run_id: int,
    source_set: CandidateSourceSet,
    run_root: Path,
    evidence_root: Path,
    runtime: CandidateRuntimePaths,
) -> CandidateRun:
    captured_sources: list[CapturedSource] = []
    browser_version: str | None = None
    for source in source_set.sources:
        source_root = run_root / source.track / source.source_id
        conversion = run_conversion(
            runtime.converter,
            source.path,
            source_set.document_format,
            source_root / "conversion",
            soffice=runtime.soffice,
            pdftohtml=runtime.pdftohtml,
            pdfinfo=runtime.pdfinfo,
            timeout_seconds=runtime.timeout_seconds,
        )
        if conversion.source_sha256 != source.source_sha256:
            raise CandidateRunError(
                f"source hash differs from frozen corpus: {source.source_id}"
            )
        captured = capture_html_units(
            conversion.html,
            source_set.document_format,
            tuple(unit.unit_id for unit in source.units),
            source_root / "artifacts",
            expected_browser_version=runtime.browser_version,
            executable_path=runtime.chromium,
            font_config=runtime.font_config,
        )
        if captured.external_requests:
            raise CandidateRunError("browser capture reported external requests")
        if browser_version is None:
            browser_version = captured.browser_version
        elif browser_version != captured.browser_version:
            raise CandidateRunError("browser version changed within a clean run")
        inventory_manifest = source_root / "inventory-manifest.json"
        write_canonical_json(
            inventory_manifest,
            {
                "schema_version": 1,
                "source_id": source.source_id,
                "unit_inventories": [
                    evidence_binding(evidence_root, unit.inventory)
                    for unit in captured.units
                ],
            },
        )
        captured_sources.append(
            CapturedSource(
                source.track,
                source.source_id,
                source.source_sha256,
                conversion.html_path,
                inventory_manifest,
                captured.units,
            )
        )
    if browser_version is None:
        raise CandidateRunError("candidate run did not capture any source")
    return CandidateRun(run_id, browser_version, tuple(captured_sources))
