from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

from evaluate.multiformat_corpus_items import object_list, require_keys
from evaluate.multiformat_corpus_types import CorpusError
from evaluate.multiformat_metric_types import MetricError
from evaluate.multiformat_schema import (
    JsonValue,
    integer_value,
    object_value,
    sha256_file,
    sha256_value,
    string_value,
)
from evaluate.multiformat_strict_json import read_strict_object


class OfficeOracleBatchError(Exception):
    pass


@dataclass(frozen=True, slots=True)
class OfficeOracleBatchUnit:
    png: Path
    width: int
    height: int


@dataclass(frozen=True, slots=True)
class OfficeOracleBatchFile:
    source_id: str
    document_format: str
    source_sha256: str
    pdf: Path
    semantic: Path
    layout: Path
    units: tuple[OfficeOracleBatchUnit, ...]


@dataclass(frozen=True, slots=True)
class OfficeOracleBatch:
    manifest: Path
    batch_id: str
    capture_timestamp: str
    golden_set_revision: str
    font_bundle_sha256: str
    runtime: dict[str, JsonValue]
    files: dict[str, OfficeOracleBatchFile]
    artifacts: frozenset[Path]


def load_office_oracle_batch(path: Path) -> OfficeOracleBatch:
    try:
        manifest = path.resolve(strict=True)
        root = manifest.parent
        values = read_strict_object(manifest)
        require_keys(
            values,
            {
                "schema_version",
                "batch_id",
                "capture_timestamp",
                "golden_set_revision",
                "font_bundle_sha256",
                "network_isolation",
                "runtime",
                "files",
            },
            "office.batch",
        )
        if (
            integer_value(values, "schema_version") != 2
            or string_value(values, "network_isolation") != "disabled"
        ):
            raise OfficeOracleBatchError("office batch is not trusted")
        runtime = object_value(values, "runtime")
        _validate_runtime(runtime)
        artifacts: set[Path] = set()
        files: dict[str, OfficeOracleBatchFile] = {}
        for item in object_list(values, "files", "office.batch.files"):
            parsed = _parse_file(item, root, artifacts)
            if parsed.source_id in files:
                raise OfficeOracleBatchError("office batch source is duplicated")
            files[parsed.source_id] = parsed
        if not files:
            raise OfficeOracleBatchError("office batch has no sources")
        expected_files = {*artifacts, manifest}
        actual_files = {
            item for item in root.rglob("*") if item.is_file() or item.is_symlink()
        }
        if actual_files != expected_files:
            raise OfficeOracleBatchError("office batch file set is not exact")
        return OfficeOracleBatch(
            manifest,
            string_value(values, "batch_id"),
            string_value(values, "capture_timestamp"),
            string_value(values, "golden_set_revision"),
            sha256_value(values, "font_bundle_sha256"),
            runtime,
            files,
            frozenset(artifacts),
        )
    except OfficeOracleBatchError:
        raise
    except (CorpusError, MetricError, OSError, TypeError, ValueError) as error:
        raise OfficeOracleBatchError("office batch is invalid") from error


def _parse_file(
    values: dict[str, JsonValue],
    root: Path,
    artifacts: set[Path],
) -> OfficeOracleBatchFile:
    require_keys(
        values,
        {
            "id",
            "format",
            "source_sha256",
            "pdf",
            "semantic",
            "layout",
            "visual_units",
        },
        "office.batch.file",
    )
    units = tuple(
        _parse_unit(item, root, artifacts)
        for item in object_list(
            values,
            "visual_units",
            "office.batch.visual_units",
        )
    )
    if not units:
        raise OfficeOracleBatchError("office batch source has no units")
    return OfficeOracleBatchFile(
        string_value(values, "id"),
        string_value(values, "format"),
        sha256_value(values, "source_sha256"),
        _artifact_path(values, "pdf", root, artifacts),
        _artifact_path(values, "semantic", root, artifacts),
        _artifact_path(values, "layout", root, artifacts),
        units,
    )


def _parse_unit(
    values: dict[str, JsonValue],
    root: Path,
    artifacts: set[Path],
) -> OfficeOracleBatchUnit:
    require_keys(
        values,
        {"png", "width", "height"},
        "office.batch.unit",
    )
    width = integer_value(values, "width")
    height = integer_value(values, "height")
    if width <= 0 or height <= 0:
        raise OfficeOracleBatchError("office batch unit dimensions are invalid")
    return OfficeOracleBatchUnit(
        _artifact_path(values, "png", root, artifacts),
        width,
        height,
    )


def _artifact_path(
    values: dict[str, JsonValue],
    field: str,
    root: Path,
    artifacts: set[Path],
) -> Path:
    binding = object_value(values, field)
    require_keys(binding, {"path", "sha256"}, f"office.batch.{field}")
    relative_value = string_value(binding, "path")
    relative = Path(relative_value)
    if (
        relative.is_absolute()
        or "\\" in relative_value
        or relative_value != relative.as_posix()
        or any(part in {"", ".", ".."} for part in relative.parts)
    ):
        raise OfficeOracleBatchError("office batch artifact path is unsafe")
    candidate = root / relative
    if candidate.is_symlink():
        raise OfficeOracleBatchError("office batch artifact is a symlink")
    resolved = candidate.resolve(strict=True)
    if (
        not resolved.is_relative_to(root)
        or not resolved.is_file()
        or sha256_file(resolved) != sha256_value(binding, "sha256")
        or resolved in artifacts
    ):
        raise OfficeOracleBatchError("office batch artifact is invalid")
    artifacts.add(resolved)
    return resolved


def _validate_runtime(values: dict[str, JsonValue]) -> None:
    require_keys(
        values,
        {
            "windows",
            "architecture",
            "office_channel",
            "word",
            "excel",
            "powerpoint",
            "pdf_primary",
            "pdf_secondary",
            "pdf_text",
        },
        "office.batch.runtime",
    )
    for field in values:
        string_value(values, field)
