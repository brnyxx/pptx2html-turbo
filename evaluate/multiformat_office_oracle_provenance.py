from __future__ import annotations

from collections import defaultdict
from pathlib import Path

from evaluate.multiformat_corpus_items import require_keys
from evaluate.multiformat_metric_compute import resolve_artifact_binding
from evaluate.multiformat_metric_types import MetricError
from evaluate.multiformat_office_oracle_batch import (
    OfficeOracleBatch,
    OfficeOracleBatchError,
    load_office_oracle_batch,
)
from evaluate.multiformat_office_oracle_receipt import (
    OfficeOracleReceiptError,
    validate_office_oracle_receipt,
)
from evaluate.multiformat_office_oracle_runtime import (
    validate_office_oracle_runtime,
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


def office_oracle_bindings(
    values: dict[str, JsonValue],
    role: str,
    oracle_lock_path: Path | None,
) -> tuple[dict[str, JsonValue] | None, dict[str, JsonValue] | None]:
    if role != "oracle" or oracle_lock_path is None:
        return None, None
    return (
        object_value(values, "execution_receipt"),
        object_value(values, "office_batch_manifest"),
    )


def validate_office_oracle_provenance(
    *,
    receipt_binding: dict[str, JsonValue],
    batch_binding: dict[str, JsonValue],
    units: JsonValue,
    runtime_path: Path,
    execution_path: Path,
    producer: str,
    project_revision: str,
    contract_hash: str,
    corpus_hash: str,
    evaluator_hash: str,
    oracle_hash: str,
    evidence_root: Path,
    oracle_lock_path: Path,
) -> None:
    receipt_path = resolve_artifact_binding(
        receipt_binding,
        evidence_root,
        "capture.execution_receipt",
    )
    batch_path = resolve_artifact_binding(
        batch_binding,
        evidence_root,
        "capture.office_batch_manifest",
    )
    runtime_artifacts = validate_office_oracle_runtime(
        runtime_path,
        oracle_lock_path,
        evidence_root,
        producer,
    )
    runtime_values = read_strict_object(runtime_path)
    try:
        batch = load_office_oracle_batch(batch_path)
        _validate_batch_units(batch, units, evidence_root)
        validate_office_oracle_receipt(
            receipt=receipt_path,
            public_key=runtime_artifacts["office_oracle_public_key"],
            openssl=runtime_artifacts["openssl_binary"],
            oracle_lock=oracle_lock_path,
            run_nonce=string_value(
                object_value(runtime_values, "tools"),
                "run_nonce",
            ),
            project_revision=project_revision,
            contract_sha256=contract_hash,
            corpus_sha256=corpus_hash,
            evaluator_sha256=evaluator_hash,
            oracle_lock_sha256=oracle_hash,
            batch_manifest=batch_path,
            runtime_identity=runtime_path,
            execution_log=execution_path,
            artifacts=_oracle_artifacts(
                units,
                runtime_artifacts,
                batch.artifacts,
                evidence_root,
            ),
            evidence_root=evidence_root,
        )
    except (OfficeOracleBatchError, OfficeOracleReceiptError) as error:
        raise MetricError(
            "metrics.binding.capture",
            "office oracle receipt",
        ) from error


def _validate_batch_units(
    batch: OfficeOracleBatch,
    value: JsonValue,
    evidence_root: Path,
) -> None:
    if not isinstance(value, list) or any(not isinstance(item, dict) for item in value):
        raise MetricError("metrics.binding.capture", "invalid oracle units")
    by_source: defaultdict[str, list[dict[str, JsonValue]]] = defaultdict(list)
    for item in value:
        require_keys(
            item,
            {
                "unit_id",
                "source_id",
                "source_sha256",
                "ordinal",
                "png",
                "inventory",
            },
            "office.oracle.unit",
        )
        by_source[string_value(item, "source_id")].append(item)
    if set(by_source) != set(batch.files):
        raise MetricError("metrics.binding.capture", "office batch source set")
    for source_id, items in by_source.items():
        batch_file = batch.files[source_id]
        ordered = sorted(items, key=lambda item: integer_value(item, "ordinal"))
        if len(ordered) != len(batch_file.units) or any(
            sha256_value(item, "source_sha256") != batch_file.source_sha256
            for item in ordered
        ):
            raise MetricError("metrics.binding.capture", "office batch unit set")
        for item, batch_unit in zip(ordered, batch_file.units, strict=True):
            png = resolve_artifact_binding(
                object_value(item, "png"),
                evidence_root,
                "office.oracle.png",
            )
            if sha256_file(png) != sha256_file(batch_unit.png):
                raise MetricError(
                    "metrics.binding.capture",
                    "office batch png set",
                )


def _oracle_artifacts(
    value: JsonValue,
    runtime_artifacts: dict[str, Path],
    batch_artifacts: frozenset[Path],
    evidence_root: Path,
) -> list[Path]:
    if not isinstance(value, list) or any(not isinstance(item, dict) for item in value):
        raise MetricError("metrics.binding.capture", "invalid oracle units")
    artifacts = {*runtime_artifacts.values(), *batch_artifacts}
    for item in value:
        for field in ["png", "inventory"]:
            artifacts.add(
                resolve_artifact_binding(
                    object_value(item, field),
                    evidence_root,
                    f"office.oracle.{field}",
                )
            )
    return sorted(artifacts, key=lambda item: item.as_posix())
