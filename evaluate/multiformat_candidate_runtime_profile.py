from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

from evaluate.multiformat_evidence import resolve_evidence_path
from evaluate.multiformat_portable_lock import validate_reference_lock
from evaluate.multiformat_reference_profile import ReferenceProfile
from evaluate.multiformat_schema import (
    JsonValue,
    integer_value,
    object_value,
    sha256_file,
    sha256_value,
    string_value,
)
from evaluate.multiformat_strict_json import read_strict_object


class CandidateRuntimeProfileError(ValueError):
    """A candidate runtime lock cannot be resolved without ambiguity."""


@dataclass(frozen=True, slots=True)
class CandidateRuntimeProfile:
    schema_version: int
    profile: ReferenceProfile
    browser_lock: dict[str, JsonValue]
    candidate_runtime_lock: dict[str, JsonValue]
    sandbox_verifier: dict[str, JsonValue]
    font_bundle: Path | None
    chromium: Path | None
    receipt_executor: Path | None
    receipt_public_key: Path | None
    attestation: Path | None
    browser_version: str
    routing_sha256: str | None
    project_revision: str

    signer_id: str

    @property
    def portable(self) -> bool:
        return self.schema_version == 2


def resolve_candidate_runtime_profile(
    lock_path: Path,
    evidence_root: Path,
    contract_path: Path,
    corpus_path: Path,
    evaluator_path: Path,
    project_revision: str,
) -> CandidateRuntimeProfile:
    """Resolve legacy inline locks or schema-2 bound runtime artifacts."""
    try:
        lock = read_strict_object(lock_path)
        schema_version = integer_value(lock, "schema_version")
        if schema_version == 1:
            browser = object_value(lock, "browser")
            runtime = object_value(lock, "candidate_runtime")
            return CandidateRuntimeProfile(
                1,
                ReferenceProfile.MICROSOFT_OFFICE,
                browser,
                runtime,
                object_value(lock, "sandbox_verifier"),
                None,
                None,
                None,
                None,
                None,
                string_value(browser, "chromium"),
                None,
                string_value(runtime, "build_revision"),
                string_value(object_value(lock, "sandbox_verifier"), "verifier_id"),
            )
        if schema_version != 2:
            raise CandidateRuntimeProfileError("candidate lock schema is unsupported")
        identity = validate_reference_lock(lock_path, evidence_root)
        if identity.profile is not ReferenceProfile.LIBREOFFICE_POPPLER:
            raise CandidateRuntimeProfileError("candidate lock profile is unsupported")
        scope = object_value(lock, "scope")
        expected_scope = {
            "contract": contract_path,
            "corpus": corpus_path,
            "evaluator": evaluator_path,
        }
        for field, expected in expected_scope.items():
            actual = _bound_path(object_value(scope, field), evidence_root)
            if actual != expected.resolve(strict=True):
                raise CandidateRuntimeProfileError(
                    f"candidate portable scope differs: {field}"
                )
        revision = string_value(scope, "project_revision")
        if revision != project_revision:
            raise CandidateRuntimeProfileError(
                "candidate portable scope differs: project revision"
            )
        browser = object_value(lock, "browser")
        browser_lock = read_strict_object(
            _bound_path(object_value(browser, "lock"), evidence_root)
        )
        runtime_lock = read_strict_object(
            _bound_path(object_value(lock, "candidate_runtime_lock"), evidence_root)
        )
        if integer_value(browser_lock, "schema_version") != 1:
            raise CandidateRuntimeProfileError("candidate browser lock schema differs")
        if integer_value(runtime_lock, "schema_version") != 1:
            raise CandidateRuntimeProfileError("candidate runtime lock schema differs")
        sandbox_verifier = object_value(runtime_lock, "sandbox_verifier")
        chromium_binding = object_value(browser, "chromium")
        browser_version = string_value(chromium_binding, "version")
        if string_value(browser_lock, "chromium") != browser_version:
            raise CandidateRuntimeProfileError("candidate Chromium versions differ")
        signer = object_value(lock, "signer")
        return CandidateRuntimeProfile(
            2,
            identity.profile,
            browser_lock,
            runtime_lock,
            sandbox_verifier,
            _bound_path(object_value(lock, "font_bundle"), evidence_root),
            _bound_path(chromium_binding, evidence_root),
            _bound_path(object_value(signer, "executor"), evidence_root),
            _bound_path(object_value(signer, "public_key"), evidence_root),
            _bound_path(
                object_value(object_value(lock, "runtime"), "attestation"),
                evidence_root,
            ),
            browser_version,
            identity.routing.sha256,
            revision,
            string_value(signer, "signer_id"),
        )
    except CandidateRuntimeProfileError:
        raise
    except (OSError, TypeError, UnicodeError, ValueError) as error:
        raise CandidateRuntimeProfileError(
            "candidate runtime profile is invalid"
        ) from error


def legacy_candidate_runtime_profile(lock_path: Path) -> CandidateRuntimeProfile:
    """Adapt the established schema-1 manifest-production boundary."""
    lock = read_strict_object(lock_path)
    browser = object_value(lock, "browser")
    runtime = object_value(lock, "candidate_runtime")
    return CandidateRuntimeProfile(
        schema_version=1,
        profile=ReferenceProfile.MICROSOFT_OFFICE,
        browser_lock=browser,
        candidate_runtime_lock=runtime,
        sandbox_verifier=object_value(lock, "sandbox_verifier"),
        font_bundle=None,
        chromium=None,
        receipt_executor=None,
        receipt_public_key=None,
        attestation=None,
        browser_version=string_value(browser, "chromium"),
        routing_sha256=None,
        project_revision=string_value(runtime, "build_revision"),
        signer_id=string_value(object_value(lock, "sandbox_verifier"), "verifier_id"),
    )


def require_profile_path(supplied: Path, locked: Path | None, label: str) -> Path:
    """Require a caller path to name the exact schema-2 bound artifact."""
    resolved = supplied.resolve(strict=True)
    if locked is not None and resolved != locked:
        raise CandidateRuntimeProfileError(f"candidate {label} path differs from lock")
    return resolved


def _bound_path(binding: dict[str, JsonValue], evidence_root: Path) -> Path:
    path = resolve_evidence_path(evidence_root, string_value(binding, "path"))
    if sha256_file(path) != sha256_value(binding, "sha256"):
        raise CandidateRuntimeProfileError("candidate bound artifact digest differs")
    return path


__all__ = [
    "CandidateRuntimeProfile",
    "CandidateRuntimeProfileError",
    "legacy_candidate_runtime_profile",
    "require_profile_path",
    "resolve_candidate_runtime_profile",
]
