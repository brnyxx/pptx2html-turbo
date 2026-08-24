"""Validated portable-lock trust loader for signed receipts."""

from __future__ import annotations

import hashlib
from pathlib import Path
from typing import TypeAlias

from evaluate.jcs import canonicalize
from evaluate.multiformat_evidence import resolve_evidence_path
from evaluate.multiformat_portable_lock import validate_reference_lock
from evaluate.multiformat_portable_receipt_context import (
    _TRUST_SEAL,
    EnvironmentIdentity,
    PortableReceiptTrustContext,
    PortableReceiptTrustError,
    ToolIdentity,
    runtime_record,
    scope_record,
)
from evaluate.multiformat_portable_receipt_validation import (
    ReceiptValidationError,
    StableFileIdentity,
    reject_identity_aliases,
    verify_stable_file,
)
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

JsonObject: TypeAlias = dict[str, JsonValue]

__all__ = [
    "PortableReceiptTrustContext",
    "PortableReceiptTrustError",
    "load_portable_receipt_trust",
    "runtime_record",
    "verify_trusted_files",
]


def load_portable_receipt_trust(
    lock_path: Path,
    evidence_root: Path,
) -> PortableReceiptTrustContext:
    """Validate a portable lock and retain every stable lock-bound identity."""
    try:
        identity = validate_reference_lock(lock_path, evidence_root)
        if identity.profile is not ReferenceProfile.LIBREOFFICE_POPPLER:
            raise PortableReceiptTrustError("portable receipt profile is not trusted")
        lock = read_strict_object(lock_path)
        if sha256_file(lock_path) != identity.sha256:
            raise PortableReceiptTrustError("portable lock changed after validation")
        lock_artifacts = _lock_artifacts(
            lock_path,
            lock,
            evidence_root,
            identity.sha256,
        )
        reject_identity_aliases((lock_artifacts,))
        by_role = {artifact.role: artifact for artifact in lock_artifacts}
        public_key_identity = by_role["public-key"]
        public_key = (evidence_root / public_key_identity.path).read_bytes()
        if (
            len(public_key) != 32
            or hashlib.sha256(public_key).hexdigest() != public_key_identity.sha256
        ):
            raise PortableReceiptTrustError("portable receipt public key is invalid")
        corpus_identity = by_role["corpus-manifest"]
        sources = _load_sources(
            evidence_root / corpus_identity.path,
            evidence_root,
        )
        reject_identity_aliases((lock_artifacts, sources))
        scope = object_value(lock, "scope")
        signer = object_value(lock, "signer")
        platform = object_value(lock, "platform")
        canonicalizer = object_value(lock, "canonicalizer")
        runtime = object_value(lock, "runtime")
        tools = _tools(lock)
        environment = EnvironmentIdentity(
            locale=string_value(runtime, "locale"),
            timezone=string_value(runtime, "timezone"),
            rendering_dpi=integer_value(runtime, "rendering_dpi"),
            attestation_sha256=by_role["attestation"].sha256,
        )
        values = {
            "lock_sha256": identity.sha256,
            "routing_sha256": identity.routing.sha256,
            "contract_sha256": by_role["contract"].sha256,
            "corpus_sha256": corpus_identity.sha256,
            "evaluator_sha256": by_role["evaluator"].sha256,
            "project_revision": string_value(scope, "project_revision"),
            "platform_os": string_value(platform, "os"),
            "architecture": string_value(platform, "architecture"),
            "executor_sha256": by_role["executor"].sha256,
            "signer_identity": string_value(signer, "signer_id"),
            "public_key_sha256": public_key_identity.sha256,
            "canonicalizer_version": string_value(canonicalizer, "version"),
            "canonicalizer_sha256": by_role["canonicalizer"].sha256,
        }
        scope_sha256 = hashlib.sha256(
            canonicalize(scope_record(values, tools, environment, sources))
        ).hexdigest()
        return PortableReceiptTrustContext(
            evidence_root=evidence_root.resolve(strict=True),
            public_key=public_key,
            tools=tools,
            environment=environment,
            sources=sources,
            lock_artifacts=lock_artifacts,
            scope_sha256=scope_sha256,
            _seal=_TRUST_SEAL,
            **values,
        )
    except PortableReceiptTrustError:
        raise
    except ReceiptValidationError as error:
        raise PortableReceiptTrustError(str(error)) from error
    except (OSError, TypeError, UnicodeError, ValueError) as error:
        raise PortableReceiptTrustError("portable receipt trust is invalid") from error


def verify_trusted_files(
    trust: PortableReceiptTrustContext,
) -> tuple[tuple[StableFileIdentity, ...], tuple[StableFileIdentity, ...]]:
    """Reverify all lock-bound artifacts and sources from stable descriptors."""
    current_lock = tuple(
        _reverify(trust, artifact) for artifact in trust.lock_artifacts
    )
    current_sources = tuple(_reverify(trust, source) for source in trust.sources)
    if current_lock != trust.lock_artifacts or current_sources != trust.sources:
        raise PortableReceiptTrustError("portable receipt trusted identity changed")
    reject_identity_aliases((current_lock, current_sources))
    return current_lock, current_sources


def _reverify(
    trust: PortableReceiptTrustContext,
    expected: StableFileIdentity,
) -> StableFileIdentity:
    return verify_stable_file(
        trust.evidence_root,
        expected.path,
        expected.sha256,
        expected.size,
        expected.role,
    )


def _lock_artifacts(
    lock_path: Path,
    lock: JsonObject,
    root: Path,
    lock_sha256: str,
) -> tuple[StableFileIdentity, ...]:
    tools = object_value(lock, "tools")
    signer = object_value(lock, "signer")
    scope = object_value(lock, "scope")
    runtime = object_value(lock, "runtime")
    browser = object_value(lock, "browser")
    bindings = (
        ("portable-lock", _file_binding(lock_path, root, lock_sha256)),
        ("tool:libreoffice", object_value(tools, "libreoffice")),
        ("tool:poppler-render", object_value(tools, "poppler_render")),
        ("tool:poppler-text", object_value(tools, "poppler_text")),
        ("tool:poppler-metadata", object_value(tools, "poppler_metadata")),
        ("canonicalizer", object_value(lock, "canonicalizer")),
        ("font-bundle", object_value(lock, "font_bundle")),
        ("configuration", object_value(lock, "configuration")),
        ("browser:chromium", object_value(browser, "chromium")),
        ("browser:lock", object_value(browser, "lock")),
        ("candidate-runtime-lock", object_value(lock, "candidate_runtime_lock")),
        ("public-key", object_value(signer, "public_key")),
        ("executor", object_value(signer, "executor")),
        ("contract", object_value(scope, "contract")),
        ("evaluator", object_value(scope, "evaluator")),
        ("corpus-manifest", object_value(scope, "corpus")),
        ("attestation", object_value(runtime, "attestation")),
    )
    return tuple(_bound_identity(binding, root, role) for role, binding in bindings)


def _tools(lock: JsonObject) -> tuple[ToolIdentity, ...]:
    values = object_value(lock, "tools")
    records = (
        ("libreoffice", object_value(values, "libreoffice")),
        ("poppler-render", object_value(values, "poppler_render")),
        ("poppler-text", object_value(values, "poppler_text")),
        ("poppler-metadata", object_value(values, "poppler_metadata")),
        ("font-bundle", object_value(lock, "font_bundle")),
        ("configuration", object_value(lock, "configuration")),
    )
    return tuple(
        ToolIdentity(
            role, string_value(record, "version"), sha256_value(record, "sha256")
        )
        for role, record in records
    )


def _load_sources(corpus_path: Path, root: Path) -> tuple[StableFileIdentity, ...]:
    manifest = read_strict_object(corpus_path)
    raw = manifest.get("sources")
    if not isinstance(raw, list) or not raw:
        raise PortableReceiptTrustError("portable receipt corpus sources are missing")
    sources: list[StableFileIdentity] = []
    for value in raw:
        if not isinstance(value, dict):
            raise PortableReceiptTrustError("portable receipt source is invalid")
        source = resolve_evidence_path(corpus_path.parent, string_value(value, "path"))
        relative = source.relative_to(root.resolve(strict=True)).as_posix()
        sources.append(
            verify_stable_file(
                root, relative, sha256_value(value, "sha256"), None, "source"
            )
        )
    sources.sort(key=lambda item: item.path)
    reject_identity_aliases((tuple(sources),))
    return tuple(sources)


def _bound_identity(
    binding: JsonObject,
    root: Path,
    role: str,
) -> StableFileIdentity:
    return verify_stable_file(
        root,
        string_value(binding, "path"),
        sha256_value(binding, "sha256"),
        None,
        role,
    )


def _file_binding(path: Path, root: Path, digest: str) -> JsonObject:
    relative = (
        path.resolve(strict=True).relative_to(root.resolve(strict=True)).as_posix()
    )
    return {"path": relative, "sha256": digest}
