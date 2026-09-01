from __future__ import annotations

import json
from pathlib import Path

from evaluate.multiformat_command_evidence import CommandPlan, command_value
from evaluate.multiformat_schema import JsonValue
from evaluate.tests.multiformat_metric_artifact_fixture import (
    binding,
    integer_field,
    object_field,
    pair_digests,
    text_field,
)


class HardGateFixtureError(Exception):
    pass


def security_records(
    root: Path,
    sources,
    project_revision: str,
    evaluator_hash: str,
    corpus_hash: str,
    command_plan: CommandPlan,
) -> list[dict[str, JsonValue]]:
    result: list[dict[str, JsonValue]] = []
    execution_root = root / "security-executions"
    execution_root.mkdir(exist_ok=True)
    for source in sources:
        execution = execution_root / f"{source['id']}.json"
        reject = source["expected_outcome"] == "reject"
        execution.write_text(
            json.dumps(
                {
                    "schema_version": 2,
                    "status": "PASS",
                    "command_plan_sha256": command_plan.sha256,
                    "command": command_value(command_plan.security),
                    "source_id": source["id"],
                    "source_sha256": source["sha256"],
                    "case_family": source["case_family"],
                    "expected_outcome": source["expected_outcome"],
                    "observed_outcome": source["expected_outcome"],
                    "typed_error": "ExpectedReject" if reject else None,
                    "network_isolation": "disabled",
                    "external_fetches": [],
                    "active_content_executed": False,
                    "within_limits": True,
                    "project_revision": project_revision,
                    "evaluator_manifest_sha256": evaluator_hash,
                    "corpus_manifest_sha256": corpus_hash,
                },
                sort_keys=True,
            ),
            encoding="utf-8",
        )
        result.append(
            {
                "source_id": source["id"],
                "execution": binding(root, execution),
            }
        )
    return result


def determinism_run(
    root: Path,
    run_id: int,
    tracks,
    document_format: str,
    candidate_units: list[dict[str, JsonValue]],
    candidate_files: list[dict[str, JsonValue]],
) -> dict[str, JsonValue]:
    files: list[dict[str, JsonValue]] = []
    run_root = root / "determinism" / f"run-{run_id}"
    run_root.mkdir(parents=True, exist_ok=True)
    for track in ["conformance", "blind"]:
        for source in tracks[track]["items"]:
            unit_count = (
                len(source["units"]) if track == "conformance" else source["unit_count"]
            )
            stem = f"{document_format}-{track}-{source['id']}"
            candidate_file = next(
                item for item in candidate_files if item["source_id"] == source["id"]
            )
            if run_id == 1:
                html_binding = candidate_file["html"]
            else:
                candidate_html = root / text_field(
                    object_field(candidate_file, "html"), "path"
                )
                html = run_root / f"{stem}.html"
                html.write_bytes(candidate_html.read_bytes())
                html_binding = binding(root, html)
            inventory = run_root / f"{stem}.json"
            png: list[dict[str, JsonValue]] = []
            source_units = sorted(
                (unit for unit in candidate_units if unit["source_id"] == source["id"]),
                key=lambda unit: integer_field(unit, "ordinal"),
            )
            if len(source_units) != unit_count:
                raise HardGateFixtureError("candidate fixture unit mismatch")
            unit_inventories: list[dict[str, JsonValue]] = []
            for ordinal, unit in enumerate(source_units, start=1):
                if run_id == 1:
                    unit_inventories.append(object_field(unit, "inventory"))
                else:
                    candidate_inventory = root / text_field(
                        object_field(unit, "inventory"), "path"
                    )
                    run_inventory = run_root / f"{stem}-{ordinal}-inventory.json"
                    run_inventory.write_bytes(candidate_inventory.read_bytes())
                    unit_inventories.append(binding(root, run_inventory))
            inventory.write_text(
                json.dumps(
                    {
                        "schema_version": 1,
                        "source_id": source["id"],
                        "unit_inventories": unit_inventories,
                    },
                    sort_keys=True,
                ),
                encoding="utf-8",
            )
            for ordinal, unit in enumerate(source_units, start=1):
                if run_id == 1:
                    png.append(object_field(unit, "png"))
                else:
                    candidate_path = root / text_field(
                        object_field(unit, "png"), "path"
                    )
                    png_path = run_root / f"{stem}-{ordinal}.png"
                    png_path.write_bytes(candidate_path.read_bytes())
                    png.append(binding(root, png_path))
            record: dict[str, JsonValue] = {
                "track": track,
                "source_id": source["id"],
                "source_sha256": source["sha256"],
                "html": html_binding,
                "inventory": binding(root, inventory),
                "png": list(png),
            }
            files.append(record)
    result: dict[str, JsonValue] = {"run_id": run_id, "files": list(files)}
    return result


def reviewer(
    root: Path,
    document_format: str,
    reviewer_id: str,
    role: str,
    pair_ids: list[str],
    oracle_units: list[dict[str, JsonValue]],
    candidate_units: list[dict[str, JsonValue]],
    project_revision: str,
    evaluator_hash: str,
    corpus_hash: str,
) -> dict[str, JsonValue]:
    oracle = {str(unit["unit_id"]): unit for unit in oracle_units}
    candidate = {str(unit["unit_id"]): unit for unit in candidate_units}
    path = root / "reviews" / f"{document_format}-{reviewer_id}.json"
    path.parent.mkdir(exist_ok=True)
    path.write_text(
        json.dumps(
            {
                "schema_version": 1,
                "status": "PASS",
                "reviewer_id": reviewer_id,
                "reviewer_role": role,
                "independent": True,
                "checklist_version": "multiformat-review-v1",
                "project_revision": project_revision,
                "evaluator_manifest_sha256": evaluator_hash,
                "corpus_manifest_sha256": corpus_hash,
                "pairs": [
                    {
                        **pair_digests(oracle[pair_id], candidate[pair_id], pair_id),
                        "decision": "PASS",
                    }
                    for pair_id in pair_ids
                ],
            },
            sort_keys=True,
        ),
        encoding="utf-8",
    )
    return {"attestation": binding(root, path)}


def quality_evidence(
    root: Path,
    document_format: str,
    evaluator_hash: str,
    corpus_hash: str,
    project_revision: str,
    command_plan: CommandPlan,
) -> tuple[dict[str, JsonValue], dict[str, JsonValue]]:
    evidence_root = root / "quality" / document_format
    evidence_root.mkdir(parents=True, exist_ok=True)
    quality: dict[str, JsonValue] = {}
    for field in ["tests", "builds", "diagnostics", "contract_checks"]:
        path = evidence_root / f"{field}.json"
        path.write_text(
            json.dumps(
                {
                    "schema_version": 2,
                    "status": "PASS",
                    "command_id": field,
                    "command_plan_sha256": command_plan.sha256,
                    "command": command_value(command_plan.quality[field]),
                    "exit_code": 0,
                    "project_revision": project_revision,
                    "evaluator_manifest_sha256": evaluator_hash,
                    "corpus_manifest_sha256": corpus_hash,
                },
                sort_keys=True,
            ),
            encoding="utf-8",
        )
        quality[field] = binding(root, path)
    performance_path = evidence_root / "performance.json"
    performance_path.write_text(
        json.dumps(
            {
                "schema_version": 2,
                "status": "PASS",
                "within_limits": True,
                "command_plan_sha256": command_plan.sha256,
                "command": command_value(command_plan.performance),
                "project_revision": project_revision,
                "evaluator_manifest_sha256": evaluator_hash,
                "corpus_manifest_sha256": corpus_hash,
            },
            sort_keys=True,
        ),
        encoding="utf-8",
    )
    return quality, {"evidence": binding(root, performance_path)}
