from __future__ import annotations

from pathlib import Path
from typing import TypeAlias

from evaluate.multiformat_corpus_items import object_list
from evaluate.multiformat_corpus_sources import (
    resolve_source_path,
    validate_source,
)
from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_legacy_types import (
    LEGACY_PAIRS,
    LegacyConformanceError,
    LegacyPairGeneration,
    ModernSnapshot,
    ModernSource,
)
from evaluate.multiformat_schema import (
    JsonValue,
    integer_value,
    object_value,
    sha256_file,
    sha256_value,
    string_value,
)
from evaluate.multiformat_strict_json import read_strict_object

JsonObject: TypeAlias = dict[str, JsonValue]


def load_modern_snapshots(
    request: LegacyPairGeneration,
    plan: JsonObject,
) -> tuple[ModernSnapshot, ...]:
    expected_formats = {modern for _, modern in LEGACY_PAIRS}
    if len(request.modern_manifests) != len(expected_formats):
        raise LegacyConformanceError("legacy modern manifest set differs")
    snapshots = tuple(
        _load_snapshot(request, plan, manifest) for manifest in request.modern_manifests
    )
    formats = {snapshot.document_format for snapshot in snapshots}
    if len(formats) != len(snapshots) or formats != expected_formats:
        raise LegacyConformanceError("legacy modern manifest set differs")
    return tuple(sorted(snapshots, key=lambda item: item.document_format.value))


def _load_snapshot(
    request: LegacyPairGeneration,
    plan: JsonObject,
    manifest_path: Path,
) -> ModernSnapshot:
    values = read_strict_object(manifest_path)
    try:
        document_format = DocumentFormat(string_value(values, "format"))
    except ValueError as error:
        raise LegacyConformanceError("legacy modern manifest format differs") from error
    expected_formats = {modern for _, modern in LEGACY_PAIRS}
    if document_format not in expected_formats:
        raise LegacyConformanceError("legacy modern manifest set differs")
    if (
        integer_value(values, "schema_version") != 1
        or string_value(values, "status") not in {"FROZEN", "GENERATED"}
        or sha256_value(values, "contract_sha256") != sha256_file(request.contract)
        or sha256_value(values, "plan_sha256") != sha256_file(request.plan)
    ):
        raise LegacyConformanceError("legacy modern manifest binding differs")
    sources = _modern_sources(
        manifest_path,
        values,
        document_format,
        plan,
    )
    return ModernSnapshot(
        document_format,
        sha256_file(manifest_path),
        sources,
    )


def _modern_sources(
    manifest_path: Path,
    manifest: JsonObject,
    document_format: DocumentFormat,
    plan: JsonObject,
) -> tuple[ModernSource, ...]:
    cases = object_list(
        object_value(
            object_value(plan, "formats"),
            document_format.value,
        ),
        "cases",
        "legacy.modern.cases",
    )
    planned = {string_value(case, "id"): case for case in cases}
    files = object_list(manifest, "files", "legacy.modern.files")
    if len(files) != 100 or len(planned) != 100:
        raise LegacyConformanceError("legacy modern source count differs")
    result: list[ModernSource] = []
    seen: set[str] = set()
    for item in files:
        item_id = string_value(item, "id")
        case = planned.get(item_id)
        if case is None or item_id in seen:
            raise LegacyConformanceError("legacy modern source identity differs")
        seen.add(item_id)
        path = resolve_source_path(
            manifest_path.parent,
            string_value(item, "path"),
        )
        digest = sha256_value(item, "sha256")
        if sha256_file(path) != digest:
            raise LegacyConformanceError("modern source binding differs")
        if (
            integer_value(item, "ordinal") != integer_value(case, "ordinal")
            or string_value(item, "primary_stratum")
            != string_value(case, "primary_stratum")
            or integer_value(item, "unit_count") != 1
        ):
            raise LegacyConformanceError("legacy modern source identity differs")
        validate_source(
            {"id": item_id, "path": path.name, "sha256": digest},
            path.parent,
            document_format,
            require_valid_format=True,
        )
        result.append(
            ModernSource(
                item_id,
                path,
                digest,
                string_value(item, "primary_stratum"),
            )
        )
    return tuple(result)
