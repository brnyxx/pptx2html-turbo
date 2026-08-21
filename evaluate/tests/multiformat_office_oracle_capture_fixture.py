from __future__ import annotations

import json
import shutil
from collections import defaultdict
from pathlib import Path

from evaluate.multiformat_office_oracle_receipt import (
    write_office_oracle_receipt,
)
from evaluate.multiformat_schema import JsonValue, string_value
from evaluate.multiformat_strict_json import read_strict_object
from evaluate.tests.multiformat_metric_artifact_fixture import binding


class OfficeCaptureFixtureError(Exception):
    pass


def office_oracle_provenance(
    root: Path,
    document_format: str,
    units: list[dict[str, JsonValue]],
    runtime_tools: dict[str, JsonValue],
    runtime_artifacts: dict[str, JsonValue],
    runtime_identity: Path,
    execution_log: Path,
    *,
    project_revision: str,
    contract_hash: str,
    corpus_hash: str,
    evaluator_hash: str,
    oracle_hash: str,
) -> dict[str, JsonValue]:
    oracle_lock = root / "oracle-lock.json"
    if not oracle_lock.is_file():
        return {}
    batch_manifest, batch_artifacts = _write_batch(
        root,
        document_format,
        units,
        project_revision,
        oracle_lock,
    )
    artifacts = {
        _bound_path(root, unit[field])
        for unit in units
        for field in ["png", "inventory"]
    }
    artifacts.update(_bound_path(root, value) for value in runtime_artifacts.values())
    artifacts.update(batch_artifacts)
    receipt_dir = root / f"{document_format}-office-receipt"
    receipt_dir.mkdir(exist_ok=True)
    receipt = write_office_oracle_receipt(
        root,
        receipt_dir,
        _bound_path(root, runtime_artifacts["receipt_signer_binary"]),
        _bound_path(root, runtime_artifacts["office_oracle_public_key"]),
        _bound_path(root, runtime_artifacts["openssl_binary"]),
        oracle_lock,
        run_nonce=str(runtime_tools["run_nonce"]),
        project_revision=project_revision,
        contract_sha256=contract_hash,
        corpus_sha256=corpus_hash,
        evaluator_sha256=evaluator_hash,
        oracle_lock_sha256=oracle_hash,
        batch_manifest=batch_manifest,
        runtime_identity=runtime_identity,
        execution_log=execution_log,
        artifacts=sorted(artifacts, key=lambda item: item.as_posix()),
    )
    return {
        "office_batch_manifest": binding(root, batch_manifest),
        "execution_receipt": binding(root, receipt),
    }


def _write_batch(
    root: Path,
    document_format: str,
    units: list[dict[str, JsonValue]],
    project_revision: str,
    oracle_lock: Path,
) -> tuple[Path, list[Path]]:
    batch_root = root / f"{document_format}-office-batch"
    batch_root.mkdir()
    by_source: defaultdict[str, list[dict[str, JsonValue]]] = defaultdict(list)
    for unit in units:
        by_source[str(unit["source_id"])].append(unit)
    files: list[dict[str, JsonValue]] = []
    artifacts: list[Path] = []
    for source_id, source_units in sorted(by_source.items()):
        source_root = batch_root / source_id
        source_root.mkdir()
        visual_units: list[dict[str, JsonValue]] = []
        for unit in sorted(source_units, key=lambda item: int(item["ordinal"])):
            png = source_root / f"unit-{unit['ordinal']}.png"
            shutil.copy2(_bound_path(root, unit["png"]), png)
            artifacts.append(png)
            visual_units.append(
                {
                    "png": binding(batch_root, png),
                    "width": 192 if document_format not in {"ppt", "pptx"} else 960,
                    "height": 192 if document_format not in {"ppt", "pptx"} else 540,
                }
            )
        pdf = source_root / "reference.pdf"
        semantic = source_root / "semantic.json"
        layout = source_root / "layout.xml"
        pdf.write_bytes(b"%PDF-1.7\n%%EOF\n")
        semantic.write_text("{}", encoding="utf-8")
        layout.write_text(
            "<doc>"
            + "".join('<page width="192" height="192"/>' for _ in source_units)
            + "</doc>",
            encoding="utf-8",
        )
        artifacts.extend([pdf, semantic, layout])
        files.append(
            {
                "id": source_id,
                "format": document_format,
                "source_sha256": str(source_units[0]["source_sha256"]),
                "pdf": binding(batch_root, pdf),
                "semantic": binding(batch_root, semantic),
                "layout": binding(batch_root, layout),
                "visual_units": visual_units,
            }
        )
    lock = read_strict_object(oracle_lock)
    office = lock["office"]
    pdf_lock = lock["pdf"]
    if not isinstance(office, dict) or not isinstance(pdf_lock, dict):
        raise OfficeCaptureFixtureError("office lock fixture is invalid")
    batch_manifest = batch_root / "manifest.json"
    batch_manifest.write_text(
        json.dumps(
            {
                "schema_version": 2,
                "batch_id": f"{document_format}-test",
                "capture_timestamp": "2026-08-21T00:00:00Z",
                "golden_set_revision": project_revision,
                "font_bundle_sha256": string_value(
                    lock,
                    "font_bundle_sha256",
                ),
                "network_isolation": "disabled",
                "runtime": {
                    "windows": string_value(office, "os"),
                    "architecture": "test-architecture",
                    "office_channel": string_value(office, "channel"),
                    "word": string_value(office, "word"),
                    "excel": string_value(office, "excel"),
                    "powerpoint": string_value(office, "powerpoint"),
                    "pdf_primary": string_value(pdf_lock, "primary"),
                    "pdf_secondary": string_value(pdf_lock, "secondary"),
                    "pdf_text": string_value(pdf_lock, "text"),
                },
                "files": files,
            },
            sort_keys=True,
        ),
        encoding="utf-8",
    )
    return batch_manifest, artifacts


def _bound_path(root: Path, value: JsonValue) -> Path:
    if not isinstance(value, dict) or not isinstance(value.get("path"), str):
        raise OfficeCaptureFixtureError("capture fixture binding is invalid")
    return root / value["path"]
