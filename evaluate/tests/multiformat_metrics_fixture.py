from __future__ import annotations

import json
from pathlib import Path

from evaluate.multiformat_revision import current_project_revision
from evaluate.multiformat_schema import JsonValue
from evaluate.tests.multiformat_capture_fixture import (
    add_capture_units,
    write_capture_manifests,
)
from evaluate.tests.multiformat_hard_gate_fixture import (
    determinism_run,
    quality_evidence,
    reviewer,
    security_records,
)
from evaluate.tests.multiformat_metric_artifact_fixture import (
    sha256,
    write_unit_artifacts,
)

PROJECT_ROOT = Path(__file__).resolve().parents[2]


def write_metrics(
    contract: Path,
    corpus: Path,
    evaluator_hash: str,
    oracle_hash: str,
    evidence_root: Path | None = None,
) -> Path:
    root = evidence_root or corpus.parent
    corpus_value = json.loads(corpus.read_text(encoding="utf-8"))
    tracks = corpus_value["tracks"]
    document_format = corpus_value["format"]
    project_revision = current_project_revision(PROJECT_ROOT)
    corpus_hash = sha256(corpus)
    width, height = (960, 540) if document_format in {"ppt", "pptx"} else (192, 192)
    conformance: list[dict[str, JsonValue]] = []
    pair_ids: list[str] = []
    capture_units: dict[str, list[dict[str, JsonValue]]] = {
        "oracle": [],
        "candidate": [],
    }
    for source in tracks["conformance"]["items"]:
        for unit in source["units"]:
            pair_ids.append(unit["id"])
            artifacts = write_unit_artifacts(
                root,
                unit["id"],
                width,
                height,
            )
            add_capture_units(
                capture_units,
                unit["id"],
                source["id"],
                source["sha256"],
                unit["ordinal"],
                artifacts,
            )
            conformance.append(
                {
                    "source_id": source["id"],
                    "source_sha256": source["sha256"],
                    "unit_id": unit["id"],
                    "ordinal": unit["ordinal"],
                    "critical_defect": False,
                    "artifacts": artifacts,
                }
            )
    blind: list[dict[str, JsonValue]] = []
    for source in tracks["blind"]["items"]:
        units: list[dict[str, JsonValue]] = []
        for ordinal in range(1, source["unit_count"] + 1):
            unit_id = f"{source['id']}-unit-{ordinal}"
            pair_ids.append(unit_id)
            artifacts = write_unit_artifacts(
                root,
                unit_id,
                width,
                height,
            )
            add_capture_units(
                capture_units,
                unit_id,
                source["id"],
                source["sha256"],
                ordinal,
                artifacts,
            )
            units.append(
                {
                    "unit_id": unit_id,
                    "ordinal": ordinal,
                    "critical_defect": False,
                    "artifacts": artifacts,
                }
            )
        blind.append(
            {
                "source_id": source["id"],
                "source_sha256": source["sha256"],
                "critical_defect": False,
                "units": units,
            }
        )
    security = security_records(
        root,
        tracks["security"]["items"],
        project_revision,
        evaluator_hash,
        corpus_hash,
    )
    capture_bindings, candidate_files = write_capture_manifests(
        root,
        document_format,
        capture_units,
        sha256(contract),
        corpus_hash,
        evaluator_hash,
        oracle_hash,
        project_revision,
    )
    quality, performance = quality_evidence(
        root,
        document_format,
        evaluator_hash,
        corpus_hash,
        project_revision,
    )
    value: dict[str, JsonValue] = {
        "schema_version": 2,
        "status": "READY",
        "format": document_format,
        "bindings": {
            "contract_sha256": sha256(contract),
            "corpus_manifest_sha256": sha256(corpus),
            "evaluator_manifest_sha256": evaluator_hash,
            "oracle_lock_sha256": oracle_hash,
            "project_revision": project_revision,
            **capture_bindings,
        },
        "conformance": {"units": conformance},
        "blind": {"files": blind},
        "security": {"cases": security},
        "determinism": {
            "runs": [
                determinism_run(
                    root,
                    1,
                    tracks,
                    document_format,
                    capture_units["candidate"],
                    candidate_files,
                ),
                determinism_run(
                    root,
                    2,
                    tracks,
                    document_format,
                    capture_units["candidate"],
                    candidate_files,
                ),
            ]
        },
        "review": {
            "reviewers": [
                reviewer(
                    root,
                    document_format,
                    "reviewer-1",
                    "visual",
                    pair_ids,
                    capture_units["oracle"],
                    capture_units["candidate"],
                    project_revision,
                    evaluator_hash,
                    corpus_hash,
                ),
                reviewer(
                    root,
                    document_format,
                    "reviewer-2",
                    "semantic-security",
                    pair_ids,
                    capture_units["oracle"],
                    capture_units["candidate"],
                    project_revision,
                    evaluator_hash,
                    corpus_hash,
                ),
            ]
        },
        "quality": quality,
        "performance": performance,
    }
    path = root / f"{document_format}-metrics.json"
    path.write_text(json.dumps(value, sort_keys=True), encoding="utf-8")
    return path
