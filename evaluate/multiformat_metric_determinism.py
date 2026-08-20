from __future__ import annotations

from collections import Counter
from pathlib import Path

from evaluate.multiformat_corpus_items import object_list, require_keys
from evaluate.multiformat_metric_compute import resolve_artifact_binding
from evaluate.multiformat_capture_types import (
    ArtifactIdentity,
    CaptureManifest,
    CaptureUnit,
)
from evaluate.multiformat_metric_types import (
    CorpusMetricSpec,
    DeterminismSummary,
    MetricError,
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


def compute_determinism(
    values: dict[str, JsonValue],
    spec: CorpusMetricSpec,
    evidence_root: Path,
    candidate_capture: CaptureManifest,
) -> tuple[DeterminismSummary, set[Path]]:
    require_keys(values, {"runs"}, "determinism")
    runs = object_list(values, "runs", "determinism.runs")
    if len(runs) != 2:
        raise MetricError("determinism.runs", str(len(runs)))
    expected = _expected_files(spec)
    candidate_png = _candidate_png_by_file(candidate_capture.units, spec)
    maps: list[dict[tuple[str, str], tuple[str, tuple[str, ...], tuple[str, ...]]]] = []
    artifacts: set[Path] = set()
    run_ids: set[int] = set()
    for run in runs:
        require_keys(run, {"run_id", "files"}, "determinism.run")
        run_id = integer_value(run, "run_id")
        if run_id not in {1, 2} or run_id in run_ids:
            raise MetricError("determinism.runs", str(run_id))
        run_ids.add(run_id)
        actual: dict[
            tuple[str, str],
            tuple[str, tuple[str, ...], tuple[str, ...]],
        ] = {}
        for file_record in object_list(run, "files", "determinism.files"):
            require_keys(
                file_record,
                {
                    "track",
                    "source_id",
                    "source_sha256",
                    "html",
                    "inventory",
                    "png",
                },
                "determinism.file",
            )
            key = (
                string_value(file_record, "track"),
                string_value(file_record, "source_id"),
            )
            if key in actual or key not in expected:
                raise MetricError("determinism.file_set", repr(key))
            source_hash, unit_count = expected[key]
            html_binding = object_value(file_record, "html")
            html = resolve_artifact_binding(
                html_binding,
                evidence_root,
                "determinism.html",
            )
            inventory = resolve_artifact_binding(
                object_value(file_record, "inventory"),
                evidence_root,
                "determinism.inventory",
            )
            png_bindings = object_list(file_record, "png", "determinism.png")
            png = [
                resolve_artifact_binding(binding, evidence_root, "determinism.png")
                for binding in png_bindings
            ]
            new_paths = {html, inventory, *png}
            if len(new_paths) != 2 + len(png):
                raise MetricError("artifact.path", repr(key))
            if run_id == 1:
                expected_html = candidate_capture.files[key[1]].html
                if _identity(html_binding) != expected_html:
                    raise MetricError("determinism.file_set", repr(key))
                actual_candidate = tuple(
                    (
                        string_value(binding, "path"),
                        sha256_value(binding, "sha256"),
                    )
                    for binding in png_bindings
                )
                if actual_candidate != candidate_png[key]:
                    raise MetricError("determinism.file_set", repr(key))
                new_artifacts = {html, inventory}
            else:
                new_artifacts = new_paths
            inventory_hashes, inventory_paths = _inventory_hashes(
                inventory,
                candidate_capture,
                key[1],
                evidence_root,
                require_capture_bindings=run_id == 1,
            )
            if run_id == 2:
                new_artifacts.update(inventory_paths)
            if artifacts & new_artifacts:
                raise MetricError("artifact.path", repr(key))
            artifacts.update(new_artifacts)
            png_hashes = tuple(sha256_file(path) for path in png)
            if (
                sha256_value(file_record, "source_sha256") != source_hash
                or len(png_hashes) != unit_count
            ):
                raise MetricError("determinism.file_set", repr(key))
            actual[key] = (
                sha256_file(html),
                inventory_hashes,
                png_hashes,
            )
        if set(actual) != set(expected):
            raise MetricError("determinism.file_set", "missing file")
        maps.append(actual)
    return (
        DeterminismSummary(
            runs=2,
            html_hashes_equal=all(
                maps[0][key][0] == maps[1][key][0] for key in expected
            ),
            inventory_hashes_equal=all(
                maps[0][key][1] == maps[1][key][1] for key in expected
            ),
            png_hashes_equal=all(
                maps[0][key][2] == maps[1][key][2] for key in expected
            ),
        ),
        artifacts,
    )


def _expected_files(
    spec: CorpusMetricSpec,
) -> dict[tuple[str, str], tuple[str, int]]:
    conformance_counts = Counter(unit.source_id for unit in spec.conformance.values())
    conformance_hashes = {
        unit.source_id: unit.source_sha256 for unit in spec.conformance.values()
    }
    result = {
        ("conformance", source_id): (conformance_hashes[source_id], count)
        for source_id, count in conformance_counts.items()
    }
    result.update(
        {
            ("blind", source_id): (item.source_sha256, item.unit_count)
            for source_id, item in spec.blind.items()
        }
    )
    return result


def _candidate_png_by_file(
    capture: dict[str, CaptureUnit],
    spec: CorpusMetricSpec,
) -> dict[tuple[str, str], tuple[tuple[str, str], ...]]:
    grouped: dict[tuple[str, str], list[CaptureUnit]] = {}
    for unit in capture.values():
        track = "conformance" if unit.unit_id in spec.conformance else "blind"
        key = (track, unit.source_id)
        grouped.setdefault(key, []).append(unit)
    return {
        key: tuple(
            (unit.png.path, unit.png.sha256)
            for unit in sorted(units, key=lambda item: item.ordinal)
        )
        for key, units in grouped.items()
    }


def _inventory_hashes(
    path: Path,
    capture: CaptureManifest,
    source_id: str,
    evidence_root: Path,
    *,
    require_capture_bindings: bool,
) -> tuple[tuple[str, ...], set[Path]]:
    values = read_strict_object(path)
    require_keys(
        values,
        {"schema_version", "source_id", "unit_inventories"},
        "determinism.inventory",
    )
    expected = [
        unit.inventory
        for unit in sorted(
            (unit for unit in capture.units.values() if unit.source_id == source_id),
            key=lambda unit: unit.ordinal,
        )
    ]
    bindings = object_list(
        values,
        "unit_inventories",
        "determinism.unit_inventories",
    )
    actual = [_identity(binding) for binding in bindings]
    if (
        integer_value(values, "schema_version") != 1
        or string_value(values, "source_id") != source_id
        or len(actual) != len(expected)
        or (require_capture_bindings and actual != expected)
    ):
        raise MetricError("determinism.inventory", source_id)
    ordered_paths = [
        resolve_artifact_binding(
            binding,
            evidence_root,
            "determinism.unit_inventory",
        )
        for binding in bindings
    ]
    paths = set(ordered_paths)
    if len(paths) != len(ordered_paths):
        raise MetricError("artifact.path", source_id)
    return tuple(sha256_file(item) for item in ordered_paths), paths


def _identity(values: dict[str, JsonValue]) -> ArtifactIdentity:
    return ArtifactIdentity(
        string_value(values, "path"),
        sha256_value(values, "sha256"),
    )
