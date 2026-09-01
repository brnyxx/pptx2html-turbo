from __future__ import annotations

from pathlib import Path

from evaluate.multiformat_candidate_attestation import (
    CandidateAttestationError,
    attestation_scope_from_hashes,
    verify_candidate_attestation,
    verify_signed_attestation,
)
from evaluate.multiformat_candidate_browser_checks import browser_version_matches
from evaluate.multiformat_candidate_runtime_profile import (
    resolve_candidate_runtime_profile,
)
from evaluate.multiformat_capture_profile import CaptureProfileContext
from evaluate.multiformat_capture_runtime_artifacts import (
    validate_runtime_artifacts,
)
from evaluate.multiformat_corpus_items import require_keys
from evaluate.multiformat_evidence import resolve_evidence_path
from evaluate.multiformat_metric_types import MetricError
from evaluate.multiformat_office_oracle_runtime import (
    validate_office_oracle_runtime,
)
from evaluate.multiformat_portable_capture import validate_portable_runtime
from evaluate.multiformat_schema import (
    JsonValue,
    integer_value,
    object_value,
    sha256_value,
    string_value,
)
from evaluate.multiformat_strict_json import read_strict_object


def validate_capture_runtime(
    runtime_path: Path,
    role: str,
    producer: str,
    project_revision: str,
    evidence_root: Path,
    oracle_lock_path: Path | None,
    contract_hash: str,
    corpus_hash: str,
    evaluator_hash: str,
    oracle_hash: str,
    profile: CaptureProfileContext,
) -> None:
    runtime = read_strict_object(runtime_path)
    require_keys(
        runtime,
        {
            "schema_version",
            "role",
            "producer",
            "project_revision",
            "os",
            "architecture",
            "python",
            "tools",
            "artifacts",
        },
        "capture.runtime",
    )
    if (
        integer_value(runtime, "schema_version") != 1
        or string_value(runtime, "role") != role
        or string_value(runtime, "producer") != producer
        or string_value(runtime, "project_revision") != project_revision
    ):
        raise MetricError("metrics.binding.capture", f"{role} runtime")
    string_value(runtime, "os")
    string_value(runtime, "architecture")
    python_version = string_value(runtime, "python")
    if role == "candidate" and not python_version.startswith("3.11."):
        raise MetricError("metrics.binding.capture", "candidate Python")
    tools = object_value(runtime, "tools")
    if profile.portable_trust is not None:
        validate_portable_runtime(runtime, role, profile.portable_trust)
    if role != "candidate":
        if profile.is_portable:
            return
        if oracle_lock_path is not None:
            validate_office_oracle_runtime(
                runtime_path,
                oracle_lock_path,
                evidence_root,
                producer,
            )
        return
    _validate_candidate_tools(tools)
    if oracle_lock_path is None:
        return
    effective_lock = profile.candidate_lock_path or oracle_lock_path
    _validate_runtime_against_lock(
        runtime,
        tools,
        evidence_root,
        effective_lock,
        oracle_lock_path,
        contract_hash,
        corpus_hash,
        evaluator_hash,
        oracle_hash,
        project_revision,
    )


def _validate_candidate_tools(values: dict[str, JsonValue]) -> None:
    digest_fields = {
        "converter_sha256",
        "soffice_sha256",
        "pdftohtml_sha256",
        "pdfinfo_sha256",
        "chromium_sha256",
        "sandbox_attestation_sha256",
        "font_bundle_sha256",
        "font_environment_sha256",
        "font_config_sha256",
        "sandbox_public_key_sha256",
        "openssl_sha256",
        "receipt_signer_sha256",
        "runtime_package_sha256",
    }
    expected = digest_fields | {
        "playwright",
        "browser_version",
        "converter_version",
        "soffice_version",
        "pdftohtml_version",
        "pdfinfo_version",
        "receipt_signer_version",
        "build_revision",
        "sandbox_verifier_id",
        "run_nonce",
    }
    if set(values) != expected:
        raise MetricError("metrics.binding.capture", "candidate runtime tools")
    for field in digest_fields:
        sha256_value(values, field)
    if string_value(values, "playwright") != "1.62.0":
        raise MetricError("metrics.binding.capture", "candidate Playwright")
    for field in expected - digest_fields:
        string_value(values, field)
    if len(string_value(values, "build_revision")) != 40:
        raise MetricError("metrics.binding.capture", "candidate build revision")


def _validate_runtime_against_lock(
    runtime: dict[str, JsonValue],
    tools: dict[str, JsonValue],
    evidence_root: Path,
    runtime_lock_path: Path,
    outer_lock_path: Path,
    contract_hash: str,
    corpus_hash: str,
    evaluator_hash: str,
    oracle_hash: str,
    project_revision: str,
) -> None:
    artifacts = object_value(runtime, "artifacts")
    lock = read_strict_object(runtime_lock_path)
    browser = object_value(lock, "browser")
    candidate = object_value(lock, "candidate_runtime")
    sandbox = object_value(lock, "sandbox_verifier")
    artifact_paths = validate_runtime_artifacts(
        artifacts,
        tools,
        lock,
        evidence_root,
    )
    try:
        if integer_value(lock, "schema_version") == 2:
            outer = read_strict_object(outer_lock_path)
            scope = object_value(outer, "scope")
            runtime_profile = resolve_candidate_runtime_profile(
                outer_lock_path,
                evidence_root,
                _scope_path(scope, "contract", evidence_root),
                _scope_path(scope, "corpus", evidence_root),
                _scope_path(scope, "evaluator", evidence_root),
                project_revision,
            )
            attestation = verify_candidate_attestation(
                runtime_profile,
                artifact_paths["sandbox_attestation"],
                artifact_paths["sandbox_public_key"],
                artifact_paths["openssl_binary"],
                outer_lock_path,
                project_revision=project_revision,
                scope_sha256=attestation_scope_from_hashes(
                    contract_hash,
                    corpus_hash,
                    evaluator_hash,
                    oracle_hash,
                ),
            )
        else:
            attestation = verify_signed_attestation(
                artifact_paths["sandbox_attestation"],
                artifact_paths["sandbox_public_key"],
                artifact_paths["openssl_binary"],
                runtime_lock_path,
                project_revision=project_revision,
                scope_sha256=attestation_scope_from_hashes(
                    contract_hash,
                    corpus_hash,
                    evaluator_hash,
                    oracle_hash,
                ),
            )
    except CandidateAttestationError as error:
        raise MetricError("metrics.binding.capture", "sandbox signature") from error
    comparisons = [
        sha256_value(tools, "chromium_sha256")
        == sha256_value(browser, "executable_sha256"),
        string_value(tools, "playwright") == string_value(browser, "playwright"),
        browser_version_matches(
            string_value(browser, "chromium"),
            string_value(tools, "browser_version"),
        ),
        sha256_value(tools, "font_bundle_sha256")
        == sha256_value(lock, "font_bundle_sha256"),
        string_value(runtime, "os") == string_value(browser, "os"),
        string_value(runtime, "architecture") == string_value(browser, "architecture"),
        sha256_value(tools, "font_environment_sha256")
        == sha256_value(browser, "font_environment_sha256"),
        string_value(tools, "build_revision")
        == string_value(candidate, "build_revision"),
        sha256_value(tools, "sandbox_public_key_sha256")
        == sha256_value(sandbox, "public_key_sha256"),
        sha256_value(tools, "openssl_sha256")
        == sha256_value(sandbox, "openssl_sha256"),
        string_value(tools, "sandbox_verifier_id")
        == string_value(sandbox, "verifier_id"),
        string_value(tools, "run_nonce") == attestation.run_nonce,
    ]
    if not all(comparisons):
        raise MetricError("metrics.binding.capture", "candidate runtime lock")
    for name in [
        "converter",
        "soffice",
        "pdftohtml",
        "pdfinfo",
        "receipt_signer",
    ]:
        if sha256_value(tools, f"{name}_sha256") != sha256_value(
            candidate, f"{name}_sha256"
        ) or string_value(tools, f"{name}_version") != string_value(
            candidate, f"{name}_version"
        ):
            raise MetricError("metrics.binding.capture", f"candidate {name} lock")


def _scope_path(
    scope: dict[str, JsonValue],
    field: str,
    evidence_root: Path,
) -> Path:
    return resolve_evidence_path(
        evidence_root,
        string_value(object_value(scope, field), "path"),
    )
