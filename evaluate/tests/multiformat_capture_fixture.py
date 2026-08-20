from __future__ import annotations

import json
from pathlib import Path

from evaluate.multiformat_schema import JsonValue
from evaluate.tests.multiformat_metric_artifact_fixture import binding


def add_capture_units(
    values: dict[str, list[dict[str, JsonValue]]],
    unit_id: str,
    source_id: str,
    source_sha256: str,
    ordinal: int,
    artifacts: dict[str, JsonValue],
) -> None:
    for role, prefix in [("oracle", "reference"), ("candidate", "candidate")]:
        values[role].append(
            {
                "unit_id": unit_id,
                "source_id": source_id,
                "source_sha256": source_sha256,
                "ordinal": ordinal,
                "png": artifacts[f"{prefix}_png"],
                "inventory": artifacts[f"{prefix}_inventory"],
            }
        )


def write_capture_manifests(
    root: Path,
    document_format: str,
    capture_units: dict[str, list[dict[str, JsonValue]]],
    contract_hash: str,
    corpus_hash: str,
    evaluator_hash: str,
    oracle_hash: str,
    project_revision: str,
) -> tuple[dict[str, JsonValue], list[dict[str, JsonValue]]]:
    result: dict[str, JsonValue] = {}
    candidate_files = _candidate_files(
        root, document_format, capture_units["candidate"]
    )
    for role in ["oracle", "candidate"]:
        producer = _producer(role, document_format)
        runtime_hash = ("6" if role == "oracle" else "7") * 64
        execution_log = root / f"{document_format}-{role}-execution.json"
        execution_log.write_text(
            json.dumps(
                {
                    "schema_version": 1,
                    "status": "PASS",
                    "role": role,
                },
                sort_keys=True,
            ),
            encoding="utf-8",
        )
        upstream = root / f"{document_format}-{role}-upstream.json"
        upstream.write_text(
            json.dumps(
                {
                    "schema_version": 1,
                    "status": "READY",
                    "role": role,
                    "format": document_format,
                    "producer": producer,
                    "runtime_sha256": runtime_hash,
                    "project_revision": project_revision,
                    "contract_sha256": contract_hash,
                    "corpus_manifest_sha256": corpus_hash,
                    "evaluator_manifest_sha256": evaluator_hash,
                    "oracle_lock_sha256": oracle_hash,
                    "units": capture_units[role],
                    "files": candidate_files if role == "candidate" else [],
                    "execution_log": binding(root, execution_log),
                },
                sort_keys=True,
            ),
            encoding="utf-8",
        )
        path = root / f"{document_format}-{role}-capture.json"
        path.write_text(
            json.dumps(
                {
                    "schema_version": 1,
                    "status": "READY",
                    "role": role,
                    "format": document_format,
                    "producer": producer,
                    "runtime_sha256": runtime_hash,
                    "contract_sha256": contract_hash,
                    "corpus_manifest_sha256": corpus_hash,
                    "evaluator_manifest_sha256": evaluator_hash,
                    "oracle_lock_sha256": oracle_hash,
                    "network_isolation": "disabled",
                    "rendering": _rendering(document_format),
                    "upstream_manifest": binding(root, upstream),
                    "units": capture_units[role],
                    "files": candidate_files if role == "candidate" else [],
                },
                sort_keys=True,
            ),
            encoding="utf-8",
        )
        result[f"{role}_capture"] = binding(root, path)
    return result, candidate_files


def _candidate_files(
    root: Path,
    document_format: str,
    units: list[dict[str, JsonValue]],
) -> list[dict[str, JsonValue]]:
    sources: dict[str, str] = {}
    for unit in units:
        sources[str(unit["source_id"])] = str(unit["source_sha256"])
    result: list[dict[str, JsonValue]] = []
    for source_id, source_hash in sorted(sources.items()):
        html = root / "candidate-html" / f"{source_id}.html"
        html.parent.mkdir(exist_ok=True)
        html.write_text(
            f"<html data-format='{document_format}'>{source_id}</html>",
            encoding="utf-8",
        )
        result.append(
            {
                "source_id": source_id,
                "source_sha256": source_hash,
                "html": binding(root, html),
            }
        )
    return result


def _producer(role: str, document_format: str) -> str:
    if role == "candidate":
        return "document2html-candidate"
    if document_format == "pdf":
        return "locked-pdf-renderer"
    return "windows-office-native"


def _rendering(document_format: str) -> dict[str, JsonValue]:
    if document_format in {"ppt", "pptx"}:
        return {"dpi": None, "width": 960, "height": 540}
    return {"dpi": 144, "width": None, "height": None}
