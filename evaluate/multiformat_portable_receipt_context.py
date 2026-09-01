"""Immutable lock-derived context records for portable receipts."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import TypeAlias

from evaluate.multiformat_portable_receipt_validation import StableFileIdentity
from evaluate.multiformat_schema import JsonValue

JsonObject: TypeAlias = dict[str, JsonValue]


class _TrustSeal:
    __slots__ = ()


_TRUST_SEAL = _TrustSeal()


class PortableReceiptTrustError(ValueError):
    """A validated portable receipt trust context cannot be constructed."""


@dataclass(frozen=True, slots=True)
class ToolIdentity:
    role: str
    version: str
    sha256: str


@dataclass(frozen=True, slots=True)
class EnvironmentIdentity:
    locale: str
    timezone: str
    rendering_dpi: int
    attestation_sha256: str


@dataclass(frozen=True, slots=True)
class PortableReceiptTrustContext:
    evidence_root: Path
    lock_sha256: str
    routing_sha256: str
    contract_sha256: str
    corpus_sha256: str
    evaluator_sha256: str
    project_revision: str
    platform_os: str
    architecture: str
    executor_sha256: str
    signer_identity: str
    public_key: bytes
    public_key_sha256: str
    canonicalizer_version: str
    canonicalizer_sha256: str
    tools: tuple[ToolIdentity, ...]
    environment: EnvironmentIdentity
    sources: tuple[StableFileIdentity, ...]
    lock_artifacts: tuple[StableFileIdentity, ...]
    scope_sha256: str
    _seal: _TrustSeal

    def is_valid(self) -> bool:
        return self._seal is _TRUST_SEAL


def runtime_record(
    trust: PortableReceiptTrustContext,
    nonce: str,
    batch_id: str,
) -> JsonObject:
    """Build the complete signed runtime record from validated trust."""
    if not trust.is_valid():
        raise PortableReceiptTrustError("portable receipt trust identity is invalid")
    record = scope_record(
        {
            "lock_sha256": trust.lock_sha256,
            "routing_sha256": trust.routing_sha256,
            "contract_sha256": trust.contract_sha256,
            "corpus_sha256": trust.corpus_sha256,
            "evaluator_sha256": trust.evaluator_sha256,
            "project_revision": trust.project_revision,
            "platform_os": trust.platform_os,
            "architecture": trust.architecture,
            "executor_sha256": trust.executor_sha256,
            "signer_identity": trust.signer_identity,
            "public_key_sha256": trust.public_key_sha256,
            "canonicalizer_version": trust.canonicalizer_version,
            "canonicalizer_sha256": trust.canonicalizer_sha256,
        },
        trust.tools,
        trust.environment,
        trust.sources,
    )
    record["nonce"] = nonce
    record["batch_id"] = batch_id
    return record


def scope_record(
    values: dict[str, str],
    tools: tuple[ToolIdentity, ...],
    environment: EnvironmentIdentity,
    sources: tuple[StableFileIdentity, ...],
) -> JsonObject:
    return {
        "schema_version": 1,
        "receipt_schema_version": 2,
        "reference_profile": "libreoffice-poppler",
        "reference_lock": {"schema_version": 2, "sha256": values["lock_sha256"]},
        "routing_table_sha256": values["routing_sha256"],
        "corpus_sha256": values["corpus_sha256"],
        "contract_sha256": values["contract_sha256"],
        "evaluator_sha256": values["evaluator_sha256"],
        "project_revision": values["project_revision"],
        "platform": {
            "os": values["platform_os"],
            "architecture": values["architecture"],
        },
        "executor_sha256": values["executor_sha256"],
        "signer_identity": values["signer_identity"],
        "public_key_sha256": values["public_key_sha256"],
        "canonicalizer": {
            "version": values["canonicalizer_version"],
            "sha256": values["canonicalizer_sha256"],
        },
        "tools": [
            {"role": tool.role, "version": tool.version, "sha256": tool.sha256}
            for tool in tools
        ],
        "environment": {
            "locale": environment.locale,
            "timezone": environment.timezone,
            "rendering_dpi": environment.rendering_dpi,
            "network_isolation": True,
            "attestation_sha256": environment.attestation_sha256,
        },
        "sources": [
            {"path": source.path, "sha256": source.sha256, "size": source.size}
            for source in sources
        ],
    }
