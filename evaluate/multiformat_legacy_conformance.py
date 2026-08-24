from __future__ import annotations

import shutil
import stat
import tempfile
from pathlib import Path
from typing import TypeAlias

from evaluate.build_multiformat_conformance_plan import (
    ConformancePlanError,
    validate_conformance_plan,
)
from evaluate.multiformat_candidate_artifacts import write_canonical_json
from evaluate.multiformat_corpus_items import object_list
from evaluate.multiformat_corpus_sources import validate_source
from evaluate.multiformat_corpus_types import CorpusError, DocumentFormat
from evaluate.multiformat_legacy_sources import load_modern_snapshots
from evaluate.multiformat_legacy_types import (
    LEGACY_PAIRS,
    LegacyConformanceError,
    LegacyPairGeneration,
    LegacyPairJob,
    LegacyPairMaterializer,
    LegacyPairRuntime,
    LegacyToolIdentity,
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
from evaluate.multiformat_strict_json import StrictJsonError, read_strict_object

JsonObject: TypeAlias = dict[str, JsonValue]


def generate_legacy_pairs(
    request: LegacyPairGeneration,
    runtime: LegacyPairRuntime,
) -> Path:
    if request.output_dir.exists():
        raise LegacyConformanceError("legacy conformance output already exists")
    try:
        validate_conformance_plan(request.contract, request.plan)
        _validate_tools(runtime.tools)
        plan = read_strict_object(request.plan)
        snapshots = load_modern_snapshots(request, plan)
        modern = {
            snapshot.document_format: {
                source.item_id: source for source in snapshot.sources
            }
            for snapshot in snapshots
        }
        bindings: list[JsonObject] = [
            {
                "format": snapshot.document_format.value,
                "manifest_sha256": snapshot.manifest_sha256,
            }
            for snapshot in snapshots
        ]
        request.output_dir.parent.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(
            prefix=".legacy-conformance-",
            dir=request.output_dir.parent,
        ) as temp_dir:
            runtime_root = Path(temp_dir)
            staging = runtime_root / "snapshot"
            staging.mkdir()
            formats: JsonObject = {}
            for legacy_format, modern_format in LEGACY_PAIRS:
                files = _materialize_format(
                    plan,
                    legacy_format,
                    modern_format,
                    modern[modern_format],
                    staging,
                    runtime_root / "runtime",
                    runtime.materialize,
                )
                formats[legacy_format.value] = {
                    "paired_format": modern_format.value,
                    "expected_count": 60,
                    "files": files,
                }
            manifest = staging / "generation-manifest.json"
            write_canonical_json(
                manifest,
                {
                    "schema_version": 1,
                    "status": "GENERATED",
                    "contract_sha256": sha256_file(request.contract),
                    "plan_sha256": sha256_file(request.plan),
                    "tools": _tool_value(runtime.tools),
                    "modern_snapshots": bindings,
                    "formats": formats,
                },
            )
            _validate_output_set(staging, formats)
            _make_immutable(staging)
            staging.rename(request.output_dir)
        return request.output_dir / "generation-manifest.json"
    except LegacyConformanceError:
        raise
    except (
        ConformancePlanError,
        CorpusError,
        OSError,
        StrictJsonError,
        TypeError,
        ValueError,
    ) as error:
        raise LegacyConformanceError("legacy conformance generation failed") from error


def _materialize_format(
    plan: JsonObject,
    legacy_format: DocumentFormat,
    modern_format: DocumentFormat,
    modern: dict[str, ModernSource],
    staging: Path,
    runtime_root: Path,
    materialize: LegacyPairMaterializer,
) -> list[JsonObject]:
    cases = object_list(
        object_value(object_value(plan, "formats"), legacy_format.value),
        "cases",
        "legacy.cases",
    )
    paired = [case for case in cases if case.get("paired_case_id") is not None]
    if len(cases) != 100 or len(paired) != 60:
        raise LegacyConformanceError("legacy paired case count differs")
    files: list[JsonObject] = []
    for case in paired:
        case_id = string_value(case, "id")
        paired_id = string_value(case, "paired_case_id")
        source = modern.get(paired_id)
        if source is None or (
            string_value(case, "primary_stratum") != "paired-legacy"
            or string_value(case, "paired_stratum") != source.primary_stratum
        ):
            raise LegacyConformanceError("legacy pair identity differs")
        support = (
            staging
            / "support"
            / modern_format.value
            / f"{source.item_id}.{modern_format.value}"
        )
        support.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source.path, support)
        if sha256_file(support) != source.digest:
            raise LegacyConformanceError("legacy paired support copy differs")
        destination = (
            staging
            / "sources"
            / legacy_format.value
            / f"{case_id}.{legacy_format.value}"
        )
        destination.parent.mkdir(parents=True, exist_ok=True)
        unit_count = materialize(
            LegacyPairJob(
                case_id,
                legacy_format,
                support,
                destination,
                runtime_root / legacy_format.value / case_id,
            )
        )
        if unit_count != 1:
            raise LegacyConformanceError(
                "legacy paired source must have one visual unit"
            )
        digest = sha256_file(destination)
        validate_source(
            {
                "id": case_id,
                "path": destination.relative_to(staging).as_posix(),
                "sha256": digest,
            },
            staging,
            legacy_format,
            require_valid_format=True,
        )
        files.append(
            {
                "id": case_id,
                "ordinal": integer_value(case, "ordinal"),
                "primary_stratum": "paired-legacy",
                "paired_stratum": source.primary_stratum,
                "path": destination.relative_to(staging).as_posix(),
                "sha256": digest,
                "unit_count": 1,
                "paired_source": {
                    "id": source.item_id,
                    "path": support.relative_to(staging).as_posix(),
                    "sha256": source.digest,
                },
            }
        )
    return files


def _validate_tools(tools: LegacyToolIdentity) -> None:
    values = _tool_value(tools)
    for field, value in values.items():
        if not isinstance(value, str) or not value:
            raise LegacyConformanceError(f"legacy tool identity is invalid: {field}")
    for field in (
        "soffice_sha256",
        "pdfinfo_sha256",
        "font_environment_sha256",
    ):
        value = values[field]
        if len(value) != 64 or any(
            character not in "0123456789abcdef" for character in value
        ):
            raise LegacyConformanceError(f"legacy tool identity is invalid: {field}")


def _tool_value(tools: LegacyToolIdentity) -> JsonObject:
    return {
        "soffice_sha256": tools.soffice_sha256,
        "soffice_version": tools.soffice_version,
        "pdfinfo_sha256": tools.pdfinfo_sha256,
        "pdfinfo_version": tools.pdfinfo_version,
        "font_environment_sha256": tools.font_environment_sha256,
    }


def _validate_output_set(root: Path, formats: JsonObject) -> None:
    expected = {root / "generation-manifest.json"}
    for value in formats.values():
        if not isinstance(value, dict):
            raise LegacyConformanceError("legacy output format is invalid")
        for item in object_list(value, "files", "legacy.output.files"):
            source = root / string_value(item, "path")
            paired_value = object_value(item, "paired_source")
            paired = root / string_value(paired_value, "path")
            if sha256_file(source) != sha256_value(item, "sha256") or sha256_file(
                paired
            ) != sha256_value(paired_value, "sha256"):
                raise LegacyConformanceError(
                    "legacy conformance output binding differs"
                )
            expected.add(source)
            expected.add(paired)
    actual = {path for path in root.rglob("*") if path.is_file() or path.is_symlink()}
    if actual != expected:
        raise LegacyConformanceError("legacy conformance file set differs")


def _make_immutable(root: Path) -> None:
    for path in root.rglob("*"):
        if path.is_file():
            path.chmod(stat.S_IRUSR | stat.S_IRGRP | stat.S_IROTH)
