from __future__ import annotations

from pathlib import Path

from evaluate.multiformat_metric_compute import resolve_artifact_binding
from evaluate.multiformat_metric_types import MetricError
from evaluate.multiformat_schema import (
    JsonValue,
    object_value,
    sha256_file,
    sha256_value,
    string_value,
)
from evaluate.multiformat_strict_json import read_strict_object

RUNTIME_ARTIFACTS = {
    "office_oracle_public_key",
    "openssl_binary",
    "receipt_signer_binary",
}


def validate_office_oracle_runtime(
    runtime_path: Path,
    oracle_lock_path: Path,
    evidence_root: Path,
    producer: str,
) -> dict[str, Path]:
    runtime = read_strict_object(runtime_path)
    lock = read_strict_object(oracle_lock_path)
    tools = object_value(runtime, "tools")
    artifacts = object_value(runtime, "artifacts")
    if set(artifacts) != RUNTIME_ARTIFACTS:
        raise MetricError(
            "metrics.binding.capture",
            "office oracle runtime artifacts",
        )
    paths = {
        name: resolve_artifact_binding(
            object_value(artifacts, name),
            evidence_root,
            f"office.runtime.{name}",
        )
        for name in RUNTIME_ARTIFACTS
    }
    verifier = object_value(lock, "office_oracle_verifier")
    if (
        sha256_file(paths["office_oracle_public_key"])
        != sha256_value(verifier, "public_key_sha256")
        or sha256_file(paths["openssl_binary"])
        != sha256_value(verifier, "openssl_sha256")
        or sha256_file(paths["office_oracle_public_key"])
        != sha256_value(tools, "office_oracle_public_key_sha256")
        or sha256_file(paths["openssl_binary"]) != sha256_value(tools, "openssl_sha256")
        or sha256_file(paths["receipt_signer_binary"])
        != sha256_value(tools, "receipt_signer_sha256")
        or string_value(tools, "office_oracle_verifier_id")
        != string_value(verifier, "verifier_id")
        or not string_value(tools, "receipt_signer_version")
    ):
        raise MetricError(
            "metrics.binding.capture",
            "office oracle verifier lock",
        )
    if producer == "windows-office-native":
        office_lock = object_value(lock, "office")
        if string_value(runtime, "os") != string_value(office_lock, "os"):
            raise MetricError(
                "metrics.binding.capture",
                "office oracle os",
            )
        if string_value(tools, "office_channel") != string_value(
            office_lock,
            "channel",
        ):
            raise MetricError(
                "metrics.binding.capture",
                "office oracle channel",
            )
        _validate_versions(
            tools,
            office_lock,
            {
                "word": "word_version",
                "excel": "excel_version",
                "powerpoint": "powerpoint_version",
            },
        )
    elif producer == "locked-pdf-renderer":
        _validate_versions(
            tools,
            object_value(lock, "pdf"),
            {
                "primary": "pdf_primary_version",
                "secondary": "pdf_secondary_version",
                "text": "pdf_text_version",
            },
        )
    else:
        raise MetricError("metrics.binding.capture", "office oracle producer")
    return paths


def _validate_versions(
    tools: dict[str, JsonValue],
    lock: dict[str, JsonValue],
    fields: dict[str, str],
) -> None:
    for lock_field, tool_field in fields.items():
        if string_value(tools, tool_field) != string_value(lock, lock_field):
            raise MetricError(
                "metrics.binding.capture",
                f"office oracle {lock_field}",
            )
