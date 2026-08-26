from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path
from typing import TypeAlias

from evaluate.multiformat_schema import (
    JsonValue,
    boolean_value,
    integer_value,
    object_value,
    string_value,
)
from evaluate.multiformat_strict_json import read_strict_object

JsonObject: TypeAlias = dict[str, JsonValue]
ArtifactResolver = Callable[[JsonObject, Path], Path]


@dataclass(frozen=True, slots=True)
class RuntimeIdentity:
    system: str
    architecture: str
    locale: str
    timezone: str
    dpi: int


@dataclass(frozen=True, slots=True)
class SandboxBindings:
    executable: JsonObject
    profile: JsonObject


def validate_outer_sandbox(
    lock: JsonObject, evidence_root: Path, resolve_artifact: ArtifactResolver
) -> SandboxBindings:
    candidate = object_value(lock, "candidate_sandbox")
    if set(candidate) != {"public_key", "openssl", "receipt_signer"}:
        raise ValueError("portable candidate sandbox fields differ")
    for field in ("public_key", "openssl", "receipt_signer"):
        resolve_artifact(object_value(candidate, field), evidence_root)
    sandbox = object_value(lock, "sandbox")
    if set(sandbox) != {"executable", "profile"}:
        raise ValueError("portable sandbox fields differ")
    result = SandboxBindings(
        object_value(sandbox, "executable"), object_value(sandbox, "profile")
    )
    resolve_artifact(result.executable, evidence_root)
    resolve_artifact(result.profile, evidence_root)
    return result


def validate_runtime_attestation(
    path: Path,
    runtime: RuntimeIdentity,
    evidence_root: Path,
    sandbox: SandboxBindings,
    resolve_artifact: ArtifactResolver,
) -> None:
    values = read_strict_object(path)
    expected_fields = {
        "schema_version",
        "os",
        "architecture",
        "locale",
        "timezone",
        "rendering_dpi",
        "network_isolation",
        "sandbox_executable",
        "sandbox_host_artifact",
        "sandbox_profile",
    }
    if set(values) != expected_fields or integer_value(values, "schema_version") != 1:
        raise ValueError("portable attestation schema is unsupported")
    expected_strings = {
        "os": runtime.system,
        "architecture": runtime.architecture,
        "locale": runtime.locale,
        "timezone": runtime.timezone,
    }
    if any(
        string_value(values, field) != expected
        for field, expected in expected_strings.items()
    ):
        raise ValueError("portable runtime attestation does not match")
    if integer_value(values, "rendering_dpi") != runtime.dpi or not boolean_value(
        values, "network_isolation"
    ):
        raise ValueError("portable runtime attestation does not match")
    attested_executable = object_value(values, "sandbox_executable")
    attested_profile = object_value(values, "sandbox_profile")
    if attested_executable != sandbox.executable or attested_profile != sandbox.profile:
        raise ValueError("portable sandbox attestation does not match")
    resolve_artifact(attested_executable, evidence_root)
    resolve_artifact(attested_profile, evidence_root)
    resolve_artifact(object_value(values, "sandbox_host_artifact"), evidence_root)
