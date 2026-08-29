from __future__ import annotations

from pathlib import Path

from evaluate.multiformat_corpus_items import object_list
from evaluate.multiformat_metric_types import MetricError
from evaluate.multiformat_portable_receipt import (
    PortableReceiptVerification,
    verify_portable_receipt,
)
from evaluate.multiformat_portable_receipt_trust import PortableReceiptTrustContext
from evaluate.multiformat_schema import (
    JsonValue,
    object_value,
    sha256_file,
    sha256_value,
    string_value,
)


def validate_portable_runtime(
    runtime: dict[str, JsonValue],
    role: str,
    trust: PortableReceiptTrustContext,
) -> None:
    if (
        string_value(runtime, "os") != trust.platform_os
        or string_value(runtime, "architecture") != trust.architecture
    ):
        raise MetricError("metrics.binding.capture", f"{role} portable runtime")
    if role == "candidate":
        return
    expected_tools: dict[str, JsonValue] = {
        tool.role: {"version": tool.version, "sha256": tool.sha256}
        for tool in trust.tools
    }
    if object_value(runtime, "tools") != expected_tools:
        raise MetricError("metrics.binding.capture", "oracle portable tools")


def validate_portable_provenance(
    *,
    receipt_path: Path,
    values: dict[str, JsonValue],
    runtime_path: Path,
    execution_path: Path,
    evidence_root: Path,
    trust: PortableReceiptTrustContext,
    project_revision: str,
    contract_hash: str,
    corpus_hash: str,
    evaluator_hash: str,
    oracle_hash: str,
) -> None:
    expected_scope = (
        trust.project_revision,
        trust.contract_sha256,
        trust.corpus_sha256,
        trust.evaluator_sha256,
        trust.lock_sha256,
    )
    actual_scope = (
        project_revision,
        contract_hash,
        corpus_hash,
        evaluator_hash,
        oracle_hash,
    )
    if actual_scope != expected_scope:
        raise MetricError("metrics.binding.capture", "portable receipt scope")
    verified = verify_portable_receipt(
        receipt_path,
        PortableReceiptVerification(trust=trust),
    )
    expected = _capture_artifacts(
        values,
        runtime_path,
        execution_path,
        evidence_root,
    )
    actual = {
        (artifact.path, artifact.sha256, artifact.role)
        for artifact in verified.artifacts
    }
    if not expected.issubset(actual):
        raise MetricError("metrics.binding.capture", "portable receipt artifacts")


def _capture_artifacts(
    values: dict[str, JsonValue],
    runtime_path: Path,
    execution_path: Path,
    evidence_root: Path,
) -> set[tuple[str, str, str]]:
    root = evidence_root.resolve(strict=True)
    candidate = string_value(values, "role") == "candidate"
    expected = {
        _path_record(root, runtime_path, "capture-runtime-identity"),
        _path_record(root, execution_path, "capture-execution-log"),
    }
    for unit in object_list(values, "units", "capture.portable.units"):
        expected.add(_binding_record(unit, "png", "capture-unit-png"))
        expected.add(_binding_record(unit, "inventory", "capture-unit-inventory"))
    for file in object_list(values, "files", "capture.portable.files"):
        expected.add(
            _binding_record(
                file,
                "html",
                "capture-candidate-html" if candidate else "capture-html",
            )
        )
    if "determinism_manifest" in values:
        expected.add(
            _binding_record(
                values,
                "determinism_manifest",
                (
                    "capture-candidate-determinism"
                    if candidate
                    else "capture-determinism-manifest"
                ),
            )
        )
    return expected


def _binding_record(
    values: dict[str, JsonValue],
    field: str,
    role: str,
) -> tuple[str, str, str]:
    binding = object_value(values, field)
    return string_value(binding, "path"), sha256_value(binding, "sha256"), role


def _path_record(root: Path, path: Path, role: str) -> tuple[str, str, str]:
    relative = path.resolve(strict=True).relative_to(root).as_posix()
    return relative, sha256_file(path), role
