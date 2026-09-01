from __future__ import annotations

import json
from pathlib import Path

from evaluate.multiformat_candidate_sources import CandidateSourceSet
from evaluate.multiformat_candidate_types import (
    CandidateRun,
    CapturedSource,
    CapturedUnit,
)
from evaluate.multiformat_schema import JsonValue, sha256_file
from evaluate.tests.multiformat_metric_artifact_fixture import write_png


def write_candidate_run(
    root: Path,
    source_set: CandidateSourceSet,
    run_id: int,
) -> CandidateRun:
    captured_sources: list[CapturedSource] = []
    for source in source_set.sources:
        source_root = root / f"run-{run_id}" / source.track / source.source_id
        source_root.mkdir(parents=True)
        html = source_root / "document.html"
        html.write_text(f"<html>{source.source_id}</html>", encoding="utf-8")
        units: list[CapturedUnit] = []
        inventory_bindings: list[dict[str, JsonValue]] = []
        for unit in source.units:
            png = source_root / f"{unit.unit_id}.png"
            write_png(png, 100, 100, (10, 20, 30))
            inventory = source_root / f"{unit.unit_id}.json"
            inventory.write_text(
                json.dumps(
                    {
                        "schema_version": 1,
                        "unit_id": unit.unit_id,
                        "texts": [],
                        "cells": [],
                        "objects": [],
                        "unattributed_cells": [],
                    },
                    sort_keys=True,
                ),
                encoding="utf-8",
            )
            units.append(CapturedUnit(unit.unit_id, png, inventory))
            inventory_bindings.append(_binding(root, inventory))
        inventory_manifest = source_root / "inventory-manifest.json"
        inventory_manifest.write_text(
            json.dumps(
                {
                    "schema_version": 1,
                    "source_id": source.source_id,
                    "unit_inventories": inventory_bindings,
                },
                sort_keys=True,
            ),
            encoding="utf-8",
        )
        captured_sources.append(
            CapturedSource(
                source.track,
                source.source_id,
                source.source_sha256,
                html,
                inventory_manifest,
                tuple(units),
            )
        )
    return CandidateRun(run_id, "test-chromium", tuple(captured_sources))


def _binding(root: Path, path: Path) -> dict[str, JsonValue]:
    return {
        "path": path.relative_to(root).as_posix(),
        "sha256": sha256_file(path),
    }
