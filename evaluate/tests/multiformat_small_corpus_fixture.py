from __future__ import annotations

import hashlib
import json
from pathlib import Path

from evaluate.multiformat_schema import JsonValue
from evaluate.tests.multiformat_security_source_fixture import write_security_source
from evaluate.tests.multiformat_source_fixture import write_positive_source


def ready_fixture(
    root: Path,
    *,
    duplicate_blind_hash: bool = False,
    duplicate_template: bool = False,
    few_producers: bool = False,
    traversal: bool = False,
    bad_hash: bool = False,
    invalid_signature: bool = False,
    wrong_quota: bool = False,
) -> tuple[Path, Path]:
    contract = root / "contract.json"
    contract_value: dict[str, JsonValue] = {
        "schema_version": 1,
        "required_formats": ["docx"],
        "corpus": {
            "conformance_units": 2,
            "blind_files": 5,
            "security_cases": 2,
            "reviewers": 2,
            "deterministic_runs": 2,
        },
        "thresholds": {
            "format_score": 96.0,
            "visual_score": 95.0,
            "content_score": 98.0,
            "layout_score": 94.0,
            "stratum_score": 94.0,
            "minimum_unit_score": 85.0,
            "minimum_blind_file_score": 90.0,
        },
        "strata": {"docx": ["text", "tables"]},
        "stratum_quotas": {"docx": {"text": 1, "tables": 1}},
        "legacy_paired_stratum_quotas": {},
        "security_case_outcomes": {
            "docx": {
                "malformed-zip": "reject",
                "path-traversal": "reject",
            }
        },
    }
    _write_json(contract, contract_value)
    corpus_root = root / "corpus"
    sources = corpus_root / "sources"
    sources.mkdir(parents=True)

    conformance_path = sources / "conformance.docx"
    write_positive_source(conformance_path, "docx", "conformance")
    second_stratum = "text" if wrong_quota else "tables"
    conformance = {
        "id": "conformance",
        "path": "sources/conformance.docx",
        "sha256": _sha256(conformance_path),
        "paired_source": None,
        "provenance": None,
        "units": [
            {
                "id": "unit-1",
                "ordinal": 1,
                "primary_stratum": "text",
                "paired_stratum": None,
                "applicable_metrics": ["visual", "content", "layout"],
                "background": "#ffffff",
                "secondary_features": [],
            },
            {
                "id": "unit-2",
                "ordinal": 2,
                "primary_stratum": second_stratum,
                "paired_stratum": None,
                "applicable_metrics": ["visual", "content", "layout"],
                "background": "#ffffff",
                "secondary_features": [],
            },
        ],
    }

    blind: list[dict[str, JsonValue]] = []
    first_hash = ""
    for index in range(5):
        path = sources / f"blind-{index}.docx"
        if invalid_signature and index == 0:
            path.write_bytes(b"not an OOXML package")
        else:
            marker = (
                "blind-0" if duplicate_blind_hash and index == 1 else f"blind-{index}"
            )
            write_positive_source(path, "docx", marker)
        digest = _sha256(path)
        if index == 0:
            first_hash = digest
        if duplicate_blind_hash and index == 1:
            digest = first_hash
        blind.append(
            {
                "id": f"blind-{index}",
                "path": f"sources/blind-{index}.docx",
                "sha256": digest,
                "producer": (
                    f"producer-{index}" if not few_producers else "producer-0"
                ),
                "source_uri": f"https://example.test/blind-{index}.docx",
                "template_family": (
                    "template-0"
                    if duplicate_template and index == 1
                    else f"template-{index}"
                ),
                "unit_count": 1,
                "applicable_metrics": ["visual", "content", "layout"],
                "background": "#ffffff",
            }
        )

    security: list[dict[str, JsonValue]] = []
    security_families = ("malformed-zip", "path-traversal")
    for index, family in enumerate(security_families):
        path = sources / f"security-{index}.docx"
        write_security_source(path, "docx", family)
        security.append(
            {
                "id": f"security-{index}",
                "path": f"sources/security-{index}.docx",
                "sha256": _sha256(path),
                "case_family": family,
                "expected_outcome": "reject",
            }
        )
    if traversal:
        conformance["path"] = "../escape.docx"
    if bad_hash:
        conformance["sha256"] = "0" * 64
    manifest: dict[str, JsonValue] = {
        "schema_version": 2,
        "status": "READY",
        "format": "docx",
        "contract_sha256": _sha256(contract),
        "stratum_quotas": {"text": 1, "tables": 1},
        "tracks": {
            "conformance": {"expected_count": 2, "items": [conformance]},
            "blind": {"expected_count": 5, "items": blind},
            "security": {"expected_count": 2, "items": security},
        },
    }
    manifest_path = corpus_root / "manifest.json"
    _write_json(manifest_path, manifest)
    return contract, manifest_path


def _write_json(path: Path, value: dict[str, JsonValue]) -> None:
    path.write_text(json.dumps(value, sort_keys=True), encoding="utf-8")


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()
