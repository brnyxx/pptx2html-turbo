from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Final, TypeAlias

from evaluate.multiformat_evidence import EvidencePathError, resolve_evidence_path
from evaluate.multiformat_portable_outer_sandbox import (
    RuntimeIdentity,
    validate_outer_sandbox,
    validate_runtime_attestation,
)
from evaluate.multiformat_portable_package_inventory import validate_package_binding
from evaluate.multiformat_reference_profile import (
    ReferenceLockIdentity,
    ReferenceProfile,
)
from evaluate.multiformat_reference_routing import (
    RoutingIdentity,
    load_reference_routing,
)
from evaluate.multiformat_rust_toolchain import load_locked_rust_toolchain
from evaluate.multiformat_schema import (
    JsonValue,
    boolean_value,
    integer_value,
    object_value,
    sha256_file,
    sha256_value,
    string_value,
)
from evaluate.multiformat_strict_json import read_strict_object

JsonObject: TypeAlias = dict[str, JsonValue]
_SUPPORTED_SYSTEMS: Final = frozenset({"Darwin", "Linux"})
_SUPPORTED_ARCHITECTURES: Final = frozenset({"arm64", "x86_64"})
_SIGNER_ID: Final = "multiformat-portable-reference-v1"
_ROUTING_TABLE: Final = Path(__file__).parent / "multiformat/reference-routing.v1.json"


class PortableLockError(ValueError):
    """Raised when a READY portable reference lock is invalid."""


class PortableLockIncompleteError(PortableLockError):
    """Raised when a portable reference lock has not reached READY state."""


@dataclass(frozen=True, slots=True)
class PortableReferenceLockIdentity(ReferenceLockIdentity):
    routing: RoutingIdentity
    scope_format: str
    corpus_path: Path
    corpus_sha256: str


def validate_reference_lock(
    path: Path,
    evidence_root: Path,
) -> PortableReferenceLockIdentity:
    """Validate a schema-2 portable lock and return its content identity."""
    try:
        lock = read_strict_object(path)
        if lock.get("status") == "INCOMPLETE":
            raise PortableLockIncompleteError("portable reference lock is incomplete")
        _require_identity(lock)
        platform = object_value(lock, "platform")
        system = string_value(platform, "os")
        architecture = string_value(platform, "architecture")
        if system not in _SUPPORTED_SYSTEMS:
            raise PortableLockError("portable reference OS is unsupported")
        if architecture not in _SUPPORTED_ARCHITECTURES:
            raise PortableLockError("portable reference architecture is unsupported")

        load_locked_rust_toolchain(path)
        tools = object_value(lock, "tools")
        for name in (
            "libreoffice",
            "poppler_render",
            "poppler_text",
            "poppler_metadata",
        ):
            tool = object_value(tools, name)
            string_value(tool, "version")
            executable = _artifact_path(tool, evidence_root)
            if name == "libreoffice":
                validate_package_binding(
                    tool, executable, evidence_root, _artifact_path
                )
        routing = load_reference_routing(_ROUTING_TABLE)
        if sha256_value(lock, "routing_table_sha256") != routing.sha256:
            raise PortableLockError("portable routing table digest mismatch")

        canonicalizer = object_value(lock, "canonicalizer")
        if string_value(canonicalizer, "version") != routing.canonicalizer_version:
            raise PortableLockError("portable canonicalizer version differs")
        _artifact_path(canonicalizer, evidence_root)
        for field in ("font_bundle", "configuration"):
            binding = object_value(lock, field)
            string_value(binding, "version")
            _artifact_path(binding, evidence_root)

        browser = object_value(lock, "browser")
        chromium = object_value(browser, "chromium")
        string_value(chromium, "version")
        chromium_path = _artifact_path(chromium, evidence_root)
        validate_package_binding(chromium, chromium_path, evidence_root, _artifact_path)
        _artifact_path(object_value(browser, "lock"), evidence_root)
        _artifact_path(object_value(lock, "candidate_runtime_lock"), evidence_root)

        sandbox = validate_outer_sandbox(lock, evidence_root, _artifact_path)

        signer = object_value(lock, "signer")
        if string_value(signer, "algorithm") != "ed25519":
            raise PortableLockError("portable signer algorithm is unsupported")
        if string_value(signer, "signer_id") != _SIGNER_ID:
            raise PortableLockError("portable signer identity is unsupported")
        if integer_value(signer, "receipt_schema_version") != 2:
            raise PortableLockError("portable receipt schema is unsupported")
        _artifact_path(object_value(signer, "public_key"), evidence_root)
        _artifact_path(object_value(signer, "executor"), evidence_root)

        scope = object_value(lock, "scope")
        scope_format = string_value(scope, "format")
        if scope_format not in {"pdf", "doc", "docx", "ppt", "pptx", "xls", "xlsx"}:
            raise PortableLockError("portable scope format is unsupported")
        corpus_binding = object_value(scope, "corpus")
        corpus_path = _artifact_path(corpus_binding, evidence_root)
        for field in ("contract", "evaluator"):
            _artifact_path(object_value(scope, field), evidence_root)
        _revision_value(scope, "project_revision")

        runtime = object_value(lock, "runtime")
        locale = string_value(runtime, "locale")
        timezone = string_value(runtime, "timezone")
        dpi = integer_value(runtime, "rendering_dpi")
        network_isolation = boolean_value(runtime, "network_isolation")
        if dpi != 144 or not network_isolation:
            raise PortableLockError("portable rendering isolation is unsupported")
        attestation_path = _artifact_path(
            object_value(runtime, "attestation"),
            evidence_root,
        )
        validate_runtime_attestation(
            attestation_path,
            RuntimeIdentity(system, architecture, locale, timezone, dpi),
            evidence_root,
            sandbox,
            _artifact_path,
        )
        return PortableReferenceLockIdentity(
            schema_version=2,
            profile=ReferenceProfile.LIBREOFFICE_POPPLER,
            sha256=sha256_file(path),
            routing=routing,
            scope_format=scope_format,
            corpus_path=corpus_path,
            corpus_sha256=sha256_value(corpus_binding, "sha256"),
        )
    except PortableLockIncompleteError:
        raise
    except PortableLockError:
        raise
    except (EvidencePathError, OSError, TypeError, UnicodeError, ValueError) as error:
        raise PortableLockError("portable reference lock is invalid") from error


def portable_lock_template() -> JsonObject:
    """Return the incomplete schema-2 portable lock scaffold."""
    binding: JsonObject = {"path": "", "sha256": ""}
    versioned: JsonObject = {"version": "", **binding}
    return {
        "schema_version": 2,
        "status": "INCOMPLETE",
        "reference_profile": ReferenceProfile.LIBREOFFICE_POPPLER.value,
        "platform": {"os": "", "architecture": ""},
        "rust_toolchain": {
            "cargo": {"path": "", "sha256": ""},
            "rustc": {"path": "", "sha256": ""},
        },
        "tools": {
            "libreoffice": {**versioned},
            "poppler_render": {**versioned},
            "poppler_text": {**versioned},
            "poppler_metadata": {**versioned},
        },
        "routing_table_sha256": "",
        "canonicalizer": {**versioned},
        "font_bundle": {**versioned},
        "configuration": {**versioned},
        "browser": {"chromium": {**versioned}, "lock": {**binding}},
        "candidate_runtime_lock": {**binding},
        "candidate_sandbox": {
            "public_key": {**binding},
            "openssl": {**binding},
            "receipt_signer": {**binding},
        },
        "sandbox": {"executable": {**binding}, "profile": {**binding}},
        "signer": {
            "algorithm": "ed25519",
            "signer_id": _SIGNER_ID,
            "public_key": {**binding},
            "receipt_schema_version": 2,
            "executor": {**binding},
        },
        "scope": {
            "format": "",
            "contract": {**binding},
            "evaluator": {**binding},
            "corpus": {**binding},
            "project_revision": "",
        },
        "runtime": {
            "locale": "en-US",
            "timezone": "UTC",
            "rendering_dpi": 144,
            "network_isolation": True,
            "attestation": {**binding},
        },
    }


def _require_identity(lock: JsonObject) -> None:
    if integer_value(lock, "schema_version") != 2:
        raise PortableLockError("portable reference lock schema is unsupported")
    if string_value(lock, "status") != "locked":
        raise PortableLockError("portable reference lock is not locked")
    if string_value(lock, "reference_profile") != ReferenceProfile.LIBREOFFICE_POPPLER:
        raise PortableLockError("portable reference profile is unsupported")


def _artifact_path(binding: JsonObject, evidence_root: Path) -> Path:
    relative_path = string_value(binding, "path")
    expected = sha256_value(binding, "sha256")
    artifact = resolve_evidence_path(evidence_root, relative_path)
    if sha256_file(artifact) != expected:
        raise PortableLockError("portable lock artifact digest mismatch")
    return artifact


def _revision_value(values: JsonObject, field: str) -> str:
    revision = string_value(values, field)
    if len(revision) != 40 or any(
        character not in "0123456789abcdef" for character in revision
    ):
        raise PortableLockError("portable project revision is malformed")
    return revision
