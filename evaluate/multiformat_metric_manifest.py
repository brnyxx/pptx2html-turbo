from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

from evaluate.multiformat_candidate_artifacts import (
    evidence_binding,
    write_canonical_json,
)
from evaluate.multiformat_capture_manifest import validate_capture_manifest
from evaluate.multiformat_capture_types import ArtifactIdentity, CaptureManifest
from evaluate.multiformat_evaluator_manifest import validate_evaluator_manifest
from evaluate.multiformat_metric_links import load_metric_spec
from evaluate.multiformat_metric_types import CorpusMetricSpec, MetricError
from evaluate.multiformat_metrics import validate_metrics_evidence
from evaluate.multiformat_revision import current_project_revision
from evaluate.multiformat_schema import JsonValue, sha256_file
from evaluate.multiformat_strict_json import read_strict_object


class MetricsAssemblyError(RuntimeError):
    pass


@dataclass(frozen=True, slots=True)
class MetricContext:
    spec: CorpusMetricSpec
    oracle: CaptureManifest
    candidate: CaptureManifest
    project_revision: str
    contract_hash: str
    corpus_hash: str
    evaluator_hash: str
    oracle_hash: str
    oracle_binding: dict[str, JsonValue]
    candidate_binding: dict[str, JsonValue]
    determinism: dict[str, JsonValue]


def prepare_metric_context(
    project_root: Path,
    contract_path: Path,
    corpus_path: Path,
    evaluator_path: Path,
    oracle_lock_path: Path,
    oracle_capture_path: Path,
    candidate_capture_path: Path,
    evidence_root: Path,
) -> MetricContext:
    try:
        project_revision = current_project_revision(project_root)
        contract_hash = sha256_file(contract_path)
        corpus_hash = sha256_file(corpus_path)
        evaluator_hash = validate_evaluator_manifest(
            project_root, contract_path, evaluator_path
        )
        oracle_hash = sha256_file(oracle_lock_path)
        spec = load_metric_spec(corpus_path)
        oracle = validate_capture_manifest(
            oracle_capture_path,
            "oracle",
            spec,
            contract_hash,
            corpus_hash,
            evaluator_hash,
            oracle_hash,
            project_revision,
            evidence_root,
            oracle_lock_path,
        )
        candidate = validate_capture_manifest(
            candidate_capture_path,
            "candidate",
            spec,
            contract_hash,
            corpus_hash,
            evaluator_hash,
            oracle_hash,
            project_revision,
            evidence_root,
            oracle_lock_path,
        )
        if candidate.determinism_path is None:
            raise MetricsAssemblyError("candidate determinism binding is missing")
        return MetricContext(
            spec,
            oracle,
            candidate,
            project_revision,
            contract_hash,
            corpus_hash,
            evaluator_hash,
            oracle_hash,
            evidence_binding(evidence_root, oracle_capture_path),
            evidence_binding(evidence_root, candidate_capture_path),
            read_strict_object(candidate.determinism_path),
        )
    except MetricsAssemblyError:
        raise
    except (MetricError, OSError, TypeError, ValueError) as error:
        raise MetricsAssemblyError("capture validation failed") from error


def derive_metric_tracks(
    context: MetricContext,
    critical_defects: dict[str, bool],
) -> tuple[dict[str, JsonValue], dict[str, JsonValue]]:
    if set(critical_defects) != set(context.spec.pair_ids()):
        raise MetricsAssemblyError("critical-defect pair set is incomplete")
    conformance: list[JsonValue] = []
    for unit_id, spec in sorted(
        context.spec.conformance.items(),
        key=lambda item: (item[1].source_id, item[1].ordinal, item[0]),
    ):
        conformance.append(
            {
                "source_id": spec.source_id,
                "source_sha256": spec.source_sha256,
                "unit_id": unit_id,
                "ordinal": spec.ordinal,
                "critical_defect": critical_defects[unit_id],
                "artifacts": _artifacts(unit_id, context),
            }
        )
    blind: list[JsonValue] = []
    for source_id, spec in sorted(context.spec.blind.items()):
        units: list[JsonValue] = []
        for ordinal in range(1, spec.unit_count + 1):
            unit_id = f"{source_id}-unit-{ordinal}"
            units.append(
                {
                    "unit_id": unit_id,
                    "ordinal": ordinal,
                    "critical_defect": critical_defects[unit_id],
                    "artifacts": _artifacts(unit_id, context),
                }
            )
        blind.append(
            {
                "source_id": source_id,
                "source_sha256": spec.source_sha256,
                "critical_defect": False,
                "units": units,
            }
        )
    return {"units": conformance}, {"files": blind}


def build_metrics_manifest(
    context: MetricContext,
    conformance: dict[str, JsonValue],
    blind: dict[str, JsonValue],
    security: list[JsonValue],
    reviews: list[JsonValue],
    quality: dict[str, JsonValue],
    performance: dict[str, JsonValue],
) -> dict[str, JsonValue]:
    return {
        "schema_version": 2,
        "status": "READY",
        "format": context.spec.document_format.value,
        "bindings": {
            "contract_sha256": context.contract_hash,
            "corpus_manifest_sha256": context.corpus_hash,
            "evaluator_manifest_sha256": context.evaluator_hash,
            "oracle_lock_sha256": context.oracle_hash,
            "project_revision": context.project_revision,
            "oracle_capture": context.oracle_binding,
            "candidate_capture": context.candidate_binding,
        },
        "conformance": conformance,
        "blind": blind,
        "security": {"cases": security},
        "determinism": context.determinism,
        "review": {"reviewers": reviews},
        "quality": quality,
        "performance": performance,
    }


def publish_validated_metrics(
    value: dict[str, JsonValue],
    output_path: Path,
    context: MetricContext,
    contract_path: Path,
    corpus_path: Path,
    evidence_root: Path,
    oracle_lock_path: Path,
) -> None:
    if output_path.exists():
        raise MetricsAssemblyError("refusing to replace metrics evidence")
    output_path.parent.mkdir(parents=True, exist_ok=True)
    pending = output_path.with_name(f".{output_path.name}.pending")
    if pending.exists():
        raise MetricsAssemblyError("pending metrics evidence already exists")
    try:
        write_canonical_json(pending, value)
        validate_metrics_evidence(
            contract_path,
            corpus_path,
            pending,
            context.evaluator_hash,
            context.oracle_hash,
            evidence_root,
            oracle_lock_path,
        )
        pending.replace(output_path)
    except Exception:
        pending.unlink(missing_ok=True)
        raise


def _artifacts(unit_id: str, context: MetricContext) -> dict[str, JsonValue]:
    if unit_id not in context.oracle.units or unit_id not in context.candidate.units:
        raise MetricsAssemblyError(f"capture pair is missing: {unit_id}")
    reference = context.oracle.units[unit_id]
    candidate = context.candidate.units[unit_id]
    return {
        "reference_png": _identity(reference.png),
        "candidate_png": _identity(candidate.png),
        "reference_inventory": _identity(reference.inventory),
        "candidate_inventory": _identity(candidate.inventory),
    }


def _identity(value: ArtifactIdentity) -> dict[str, JsonValue]:
    return {"path": value.path, "sha256": value.sha256}
