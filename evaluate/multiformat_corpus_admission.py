from __future__ import annotations

import shutil
from dataclasses import dataclass, replace
from datetime import datetime, timezone
from pathlib import Path

from evaluate.jcs import JcsError, canonicalize
from evaluate.multiformat_atomic_publish import AtomicPublishError, atomic_publish
from evaluate.multiformat_contract import contract_digest
from evaluate.multiformat_corpus_admission_sources import load_admission_sources
from evaluate.multiformat_corpus_admission_types import (
    AdmissionMetadata,
    AdmissionPlan,
    AdmissionResult,
    AdmissionSource,
    AdmissionValidators,
)
from evaluate.multiformat_corpus_identity import admitted_corpus_digest
from evaluate.multiformat_corpus_types import CorpusError, CorpusStatus, DocumentFormat
from evaluate.multiformat_schema import JsonValue, object_value, sha256_file
from evaluate.multiformat_strict_json import StrictJsonError, read_strict_object

__all__ = [
    "AdmissionMetadata",
    "AdmissionPlan",
    "AdmissionResult",
    "AdmissionSource",
    "AdmissionValidators",
    "admit_corpus",
]


@dataclass(frozen=True, slots=True)
class _PublicationContext:
    plan: AdmissionPlan
    sources: tuple[AdmissionSource, ...]


def admit_corpus(
    plan: AdmissionPlan,
    validators: AdmissionValidators | None,
) -> AdmissionResult:
    """Qualify and atomically publish one seven-format corpus."""
    if validators is None:
        return AdmissionResult(CorpusStatus.INCOMPLETE, None, ("qualification",))
    try:
        _validate_plan(plan)
        context = _PublicationContext(
            plan=plan,
            sources=load_admission_sources(plan),
        )
        aggregate_digest = ""

        def write_admission(staging: Path) -> None:
            nonlocal aggregate_digest
            manifest = _materialize(staging, context, validators)
            aggregate_digest = admitted_corpus_digest(manifest)
            manifest["aggregate_sha256"] = aggregate_digest
            (staging / "manifest.json").write_bytes(canonicalize(manifest) + b"\n")

        atomic_publish(plan.destination, write_admission)
        return AdmissionResult(CorpusStatus.READY, aggregate_digest, ())
    except CorpusError as error:
        return AdmissionResult(CorpusStatus.INCOMPLETE, None, (error.reason,))
    except (
        AtomicPublishError,
        JcsError,
        OSError,
        StrictJsonError,
        TypeError,
        ValueError,
    ) as error:
        return AdmissionResult(CorpusStatus.INCOMPLETE, None, (str(error),))


def _validate_plan(plan: AdmissionPlan) -> None:
    metadata = plan.metadata
    if not metadata.corpus_revision:
        raise CorpusError("admission.corpus_revision", "must not be empty")
    if len(metadata.project_revision) != 40 or any(
        character not in "0123456789abcdef" for character in metadata.project_revision
    ):
        raise CorpusError("admission.project_revision", metadata.project_revision)
    if not metadata.admitted_at.endswith("Z"):
        raise CorpusError("admission.admitted_at", metadata.admitted_at)
    try:
        timestamp = datetime.fromisoformat(metadata.admitted_at.replace("Z", "+00:00"))
    except ValueError as error:
        raise CorpusError("admission.admitted_at", metadata.admitted_at) from error
    if timestamp.tzinfo != timezone.utc:
        raise CorpusError("admission.admitted_at", metadata.admitted_at)


def _materialize(
    staging: Path,
    context: _PublicationContext,
    validators: AdmissionValidators,
) -> dict[str, JsonValue]:
    records: list[JsonValue] = []
    for source in context.sources:
        output = staging / "sources" / source.document_format.value / source.path.name
        output.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source.path, output)
        if sha256_file(output) != source.digest:
            raise CorpusError("source.sha256", source.path.as_posix())
        staged_source = replace(source, path=output)
        evidence: dict[str, JsonValue] = {}
        for kind, check in (
            ("extraction", validators.extraction),
            ("fonts", validators.fonts),
            ("render", validators.rendering),
        ):
            proof = (
                staging
                / "admission-evidence"
                / source.document_format.value
                / f"{source.item_id}-{kind}.json"
            )
            proof.parent.mkdir(parents=True, exist_ok=True)
            proof_bytes = check(staged_source)
            if not proof_bytes:
                raise CorpusError("admission.evidence", f"{source.item_id}:{kind}")
            proof.write_bytes(proof_bytes)
            evidence[kind] = {
                "path": proof.relative_to(staging).as_posix(),
                "sha256": sha256_file(proof),
            }
        if sha256_file(output) != source.digest:
            raise CorpusError("corpus.sources_changed", output.as_posix())
        records.append(
            {
                "format": source.document_format.value,
                "id": source.item_id,
                "track": source.track,
                "path": output.relative_to(staging).as_posix(),
                "sha256": source.digest,
                "unit_count": source.unit_count,
                "evidence": evidence,
            }
        )
    contract = read_strict_object(context.plan.contract_path)
    metadata = context.plan.metadata
    return {
        "schema_version": 2,
        "status": CorpusStatus.READY.value,
        "corpus_revision": metadata.corpus_revision,
        "contract_sha256": contract_digest(context.plan.contract_path),
        "aggregate_sha256": "0" * 64,
        "per_format_counts": {
            document_format.value: {"conformance": 100, "blind": 75, "security": 10}
            for document_format in DocumentFormat
        },
        "stratum_quotas": object_value(contract, "stratum_quotas"),
        "sources": records,
        "admitted_at": metadata.admitted_at,
        "project_revision": metadata.project_revision,
    }
