from __future__ import annotations

import shutil
from pathlib import Path

from evaluate.multiformat_candidate_artifacts import write_canonical_json
from evaluate.multiformat_office_oracle_batch import (
    OfficeOracleBatch,
    OfficeOracleBatchError,
    OfficeOracleBatchFile,
    OfficeOracleBatchUnit,
    load_office_oracle_batch,
)
from evaluate.multiformat_schema import JsonValue, sha256_file


def materialize_office_oracle_batch(
    batch: OfficeOracleBatch,
    output_dir: Path,
    source_ids: set[str],
) -> OfficeOracleBatch:
    if not source_ids or not source_ids.issubset(batch.files):
        raise OfficeOracleBatchError("office batch source set is unavailable")
    target = output_dir / "batch"
    if target.exists():
        raise OfficeOracleBatchError("office batch target already exists")
    target.mkdir(parents=True)
    source_root = batch.manifest.parent
    files = [batch.files[source_id] for source_id in sorted(source_ids)]
    for source in sorted(
        _batch_artifacts(files),
        key=lambda item: item.as_posix(),
    ):
        relative = source.relative_to(source_root)
        destination = target / relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, destination)
        if sha256_file(destination) != sha256_file(source):
            raise OfficeOracleBatchError("office batch copy drifted")
    manifest = target / batch.manifest.name
    write_canonical_json(
        manifest,
        {
            "schema_version": 2,
            "batch_id": batch.batch_id,
            "capture_timestamp": batch.capture_timestamp,
            "golden_set_revision": batch.golden_set_revision,
            "font_bundle_sha256": batch.font_bundle_sha256,
            "network_isolation": "disabled",
            "runtime": batch.runtime,
            "files": [_file_value(source_root, batch_file) for batch_file in files],
        },
    )
    return load_office_oracle_batch(manifest)


def _batch_artifacts(files: list[OfficeOracleBatchFile]) -> set[Path]:
    artifacts: set[Path] = set()
    for batch_file in files:
        artifacts.update(
            {
                batch_file.pdf,
                batch_file.semantic,
                batch_file.layout,
                *(unit.png for unit in batch_file.units),
            }
        )
    return artifacts


def _file_value(
    root: Path,
    batch_file: OfficeOracleBatchFile,
) -> dict[str, JsonValue]:
    return {
        "id": batch_file.source_id,
        "format": batch_file.document_format,
        "source_sha256": batch_file.source_sha256,
        "pdf": _binding(root, batch_file.pdf),
        "semantic": _binding(root, batch_file.semantic),
        "layout": _binding(root, batch_file.layout),
        "visual_units": [_unit_value(root, unit) for unit in batch_file.units],
    }


def _unit_value(
    root: Path,
    unit: OfficeOracleBatchUnit,
) -> dict[str, JsonValue]:
    return {
        "png": _binding(root, unit.png),
        "width": unit.width,
        "height": unit.height,
    }


def _binding(root: Path, path: Path) -> dict[str, JsonValue]:
    return {
        "path": path.relative_to(root).as_posix(),
        "sha256": sha256_file(path),
    }
