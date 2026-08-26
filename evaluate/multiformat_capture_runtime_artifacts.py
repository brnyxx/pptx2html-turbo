from __future__ import annotations

import stat
from pathlib import Path

from evaluate.multiformat_candidate_fonts import (
    CandidateFontError,
    validate_font_bundle,
    validate_font_config,
)
from evaluate.multiformat_metric_compute import resolve_artifact_binding
from evaluate.multiformat_metric_types import MetricError
from evaluate.multiformat_corpus_items import object_list, require_keys
from evaluate.multiformat_schema import (
    JsonValue,
    object_value,
    sha256_file,
    sha256_value,
    string_value,
)
from evaluate.multiformat_strict_json import read_strict_object


def validate_runtime_artifacts(
    values: dict[str, JsonValue],
    tools: dict[str, JsonValue],
    lock: dict[str, JsonValue],
    evidence_root: Path,
) -> dict[str, Path]:
    expected = {
        "sandbox_attestation",
        "sandbox_public_key",
        "font_bundle",
        "font_config",
        "converter_binary",
        "soffice_binary",
        "pdftohtml_binary",
        "pdfinfo_binary",
        "chromium_binary",
        "openssl_binary",
        "receipt_signer_binary",
        "runtime_package_manifest",
    }
    if set(values) != expected:
        raise MetricError("metrics.binding.capture", "candidate runtime artifacts")
    paths = {
        name: resolve_artifact_binding(
            object_value(values, name),
            evidence_root,
            f"candidate.runtime.{name}",
        )
        for name in expected
    }
    for name, path in paths.items():
        value = path.lstat()
        if not stat.S_ISREG(value.st_mode) or path.is_symlink() or value.st_nlink != 1:
            raise MetricError("metrics.binding.capture", f"candidate {name} alias")
    comparisons = {
        "sandbox_attestation": "sandbox_attestation_sha256",
        "sandbox_public_key": "sandbox_public_key_sha256",
        "font_config": "font_config_sha256",
        "converter_binary": "converter_sha256",
        "soffice_binary": "soffice_sha256",
        "pdftohtml_binary": "pdftohtml_sha256",
        "pdfinfo_binary": "pdfinfo_sha256",
        "chromium_binary": "chromium_sha256",
        "openssl_binary": "openssl_sha256",
        "receipt_signer_binary": "receipt_signer_sha256",
        "runtime_package_manifest": "runtime_package_sha256",
    }
    for name, field in comparisons.items():
        if sha256_file(paths[name]) != sha256_value(tools, field):
            raise MetricError("metrics.binding.capture", f"candidate {name}")
    if sha256_file(paths["font_bundle"]) != sha256_value(
        lock,
        "font_bundle_sha256",
    ):
        raise MetricError("metrics.binding.capture", "candidate font bundle")
    try:
        environment_hash = validate_font_bundle(paths["font_bundle"])
        validate_font_config(
            paths["font_bundle"],
            paths["font_config"],
            evidence_root,
        )
    except CandidateFontError as error:
        raise MetricError(
            "metrics.binding.capture", "candidate font package"
        ) from error
    if environment_hash != sha256_value(tools, "font_environment_sha256"):
        raise MetricError("metrics.binding.capture", "candidate font environment")
    _validate_package_manifest(paths["runtime_package_manifest"], evidence_root)
    return paths


def _validate_package_manifest(path: Path, evidence_root: Path) -> None:
    values = read_strict_object(path)
    require_keys(values, {"schema_version", "entries"}, "runtime.package")
    entries = object_list(values, "entries", "runtime.package.entries")
    listed: set[Path] = set()
    package_roots: set[Path] = set()
    for entry in entries:
        relative = string_value(entry, "path")
        candidate = evidence_root / relative
        if not candidate.is_relative_to(evidence_root) or candidate in listed:
            raise MetricError("metrics.binding.capture", "runtime package path")
        listed.add(candidate)
        parts = Path(relative).parts
        for index, part in enumerate(parts):
            if part.endswith("-package"):
                package_roots.add(evidence_root.joinpath(*parts[: index + 1]))
                break
        require_keys(entry, {"path", "sha256"}, "runtime.package.file")
        value = candidate.lstat()
        if (
            not stat.S_ISREG(value.st_mode)
            or candidate.is_symlink()
            or value.st_nlink != 1
        ):
            raise MetricError("metrics.binding.capture", relative)
        if sha256_file(candidate) != sha256_value(entry, "sha256"):
            raise MetricError("metrics.binding.capture", relative)
    actual = {
        candidate
        for root in package_roots
        for candidate in root.rglob("*")
        if candidate.is_file()
    }
    if actual != listed:
        raise MetricError("metrics.binding.capture", "runtime package file set")
