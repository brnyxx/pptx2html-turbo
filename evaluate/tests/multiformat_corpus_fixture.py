from __future__ import annotations

import hashlib
import json
from pathlib import Path

from evaluate.multiformat_schema import (
    JsonValue,
    integer_value,
    string_value,
)
from evaluate.tests.multiformat_source_fixture import write_positive_source

PAIRED_FORMATS = {"doc": "docx", "xls": "xlsx", "ppt": "pptx"}


class CorpusFixtureError(Exception):
    pass


def write_corpus(
    evidence: Path,
    document_format: str,
    contract_hash: str,
    quota_values: dict[str, JsonValue],
    security_values: dict[str, JsonValue],
    paired_quota_values: dict[str, JsonValue] | None,
) -> Path:
    root = evidence / "corpora" / document_format
    sources = root / "sources"
    sources.mkdir(parents=True)
    quotas = {name: integer_value(quota_values, name) for name in quota_values}
    if document_format in PAIRED_FORMATS:
        if paired_quota_values is None:
            raise CorpusFixtureError("legacy corpus requires paired quotas")
        conformance = _legacy_conformance(
            sources,
            document_format,
            paired_quota_values,
        )
    else:
        conformance = _modern_conformance(sources, document_format, quotas)
    blind = _blind_sources(sources, document_format)
    security = _security_sources(sources, document_format, security_values)
    manifest: dict[str, JsonValue] = {
        "schema_version": 2,
        "status": "READY",
        "format": document_format,
        "contract_sha256": contract_hash,
        "stratum_quotas": quotas,
        "tracks": {
            "conformance": {"expected_count": 100, "items": conformance},
            "blind": {"expected_count": 75, "items": blind},
            "security": {"expected_count": 10, "items": security},
        },
    }
    manifest_path = root / "manifest.json"
    manifest_path.write_text(
        json.dumps(manifest, sort_keys=True),
        encoding="utf-8",
    )
    return manifest_path


def _modern_conformance(
    sources: Path,
    document_format: str,
    quotas: dict[str, int],
) -> list[dict[str, JsonValue]]:
    path = sources / f"conformance.{document_format}"
    write_positive_source(path, document_format, "conformance")
    units: list[dict[str, JsonValue]] = []
    ordinal = 0
    for stratum, count in quotas.items():
        for _ in range(count):
            ordinal += 1
            units.append(
                {
                    "id": f"{document_format}-unit-{ordinal}",
                    "ordinal": ordinal,
                    "primary_stratum": stratum,
                    "paired_stratum": None,
                    "applicable_metrics": ["visual", "content", "layout"],
                    "background": "#ffffff",
                    "secondary_features": [],
                }
            )
    return [
        {
            "id": f"{document_format}-conformance",
            "path": f"sources/conformance.{document_format}",
            "sha256": _sha256(path),
            "paired_source": None,
            "provenance": None,
            "units": units,
        }
    ]


def _legacy_conformance(
    sources: Path,
    document_format: str,
    paired_quota_values: dict[str, JsonValue],
) -> list[dict[str, JsonValue]]:
    paired_format = PAIRED_FORMATS[document_format]
    paired_path = sources / f"paired.{document_format}"
    modern_path = sources / f"paired-source.{paired_format}"
    binary_path = sources / f"binary.{document_format}"
    write_positive_source(paired_path, document_format, "paired")
    write_positive_source(modern_path, paired_format, "paired-source")
    write_positive_source(binary_path, document_format, "binary")
    paired_units: list[dict[str, JsonValue]] = []
    ordinal = 0
    for stratum in paired_quota_values:
        for _ in range(integer_value(paired_quota_values, stratum)):
            ordinal += 1
            paired_units.append(
                {
                    "id": f"{document_format}-paired-unit-{ordinal}",
                    "ordinal": ordinal,
                    "primary_stratum": "paired-legacy",
                    "paired_stratum": stratum,
                    "applicable_metrics": ["visual", "content", "layout"],
                    "background": "#ffffff",
                    "secondary_features": [],
                }
            )
    binary_units = [
        {
            "id": f"{document_format}-binary-unit-{ordinal}",
            "ordinal": ordinal,
            "primary_stratum": "binary-specific",
            "paired_stratum": None,
            "applicable_metrics": ["visual", "content", "layout"],
            "background": "#ffffff",
            "secondary_features": [],
        }
        for ordinal in range(1, 41)
    ]
    return [
        {
            "id": f"{document_format}-paired",
            "path": f"sources/paired.{document_format}",
            "sha256": _sha256(paired_path),
            "paired_source": {
                "id": f"{document_format}-paired-source",
                "path": f"sources/paired-source.{paired_format}",
                "sha256": _sha256(modern_path),
            },
            "provenance": None,
            "units": paired_units,
        },
        {
            "id": f"{document_format}-binary",
            "path": f"sources/binary.{document_format}",
            "sha256": _sha256(binary_path),
            "paired_source": None,
            "provenance": {
                "producer": f"independent-{document_format}",
                "source_uri": f"urn:gate-test:{document_format}:binary",
                "independently_authored": True,
            },
            "units": binary_units,
        },
    ]


def _blind_sources(
    sources: Path,
    document_format: str,
) -> list[dict[str, JsonValue]]:
    blind: list[dict[str, JsonValue]] = []
    for index in range(75):
        path = sources / f"blind-{index}.{document_format}"
        write_positive_source(path, document_format, f"blind-{index}")
        blind.append(
            {
                "id": f"{document_format}-blind-{index}",
                "path": f"sources/blind-{index}.{document_format}",
                "sha256": _sha256(path),
                "producer": f"producer-{index % 5}",
                "source_uri": f"urn:gate-test:{document_format}:{index}",
                "template_family": f"template-{index}",
                "unit_count": 1,
                "applicable_metrics": ["visual", "content", "layout"],
                "background": "#ffffff",
            }
        )
    return blind


def _security_sources(
    sources: Path,
    document_format: str,
    security_values: dict[str, JsonValue],
) -> list[dict[str, JsonValue]]:
    security: list[dict[str, JsonValue]] = []
    for index, family in enumerate(security_values):
        path = sources / f"security-{index}.{document_format}"
        outcome = string_value(security_values, family)
        if outcome == "safe-convert":
            write_positive_source(path, document_format, f"security-{family}")
        else:
            path.write_bytes(f"hostile-{document_format}-{index}".encode())
        security.append(
            {
                "id": f"{document_format}-security-{index}",
                "path": f"sources/security-{index}.{document_format}",
                "sha256": _sha256(path),
                "case_family": family,
                "expected_outcome": outcome,
            }
        )
    return security


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()
