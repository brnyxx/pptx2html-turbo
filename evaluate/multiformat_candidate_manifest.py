from __future__ import annotations

import platform
from pathlib import Path

from evaluate.multiformat_candidate_artifacts import (
    evidence_binding,
    write_canonical_json,
)
from evaluate.multiformat_candidate_determinism import (
    CandidateDeterminismError,
    determinism_run_value,
    validate_clean_runs,
)
from evaluate.multiformat_candidate_receipt import write_execution_receipt
from evaluate.multiformat_candidate_sources import CandidateSourceSet
from evaluate.multiformat_candidate_types import (
    CandidateCaptureError,
    CandidateManifestPaths,
    CandidateRun,
)
from evaluate.multiformat_capture_manifest import validate_capture_manifest
from evaluate.multiformat_metric_links import load_metric_spec
from evaluate.multiformat_metric_types import MetricError
from evaluate.multiformat_schema import JsonValue, sha256_file


class CandidateManifestError(CandidateCaptureError):
    pass


def write_candidate_manifests(
    evidence_root: Path,
    output_dir: Path,
    source_set: CandidateSourceSet,
    run1: CandidateRun,
    run2: CandidateRun,
    contract_path: Path,
    corpus_path: Path,
    evaluator_path: Path,
    oracle_lock_path: Path,
    *,
    project_revision: str,
    runtime_tools: dict[str, str],
    runtime_artifacts: dict[str, Path],
    receipt_signer: Path,
    font_bundle_sha256: str,
) -> CandidateManifestPaths:
    evidence_root = evidence_root.resolve(strict=True)
    if output_dir.exists() and any(output_dir.iterdir()):
        raise CandidateManifestError(f"candidate output is not empty: {output_dir}")
    output_dir.mkdir(parents=True, exist_ok=True)
    try:
        validate_clean_runs(source_set, run1, run2)
    except CandidateDeterminismError as error:
        raise CandidateManifestError(str(error)) from error
    contract_hash = sha256_file(contract_path)
    corpus_hash = sha256_file(corpus_path)
    evaluator_hash = sha256_file(evaluator_path)
    oracle_hash = sha256_file(oracle_lock_path)
    units, files = _capture_records(evidence_root, source_set, run1)
    runtime_identity = output_dir / "runtime-identity.json"
    runtime_value: dict[str, JsonValue] = {
        "schema_version": 1,
        "role": "candidate",
        "producer": "document2html-candidate",
        "project_revision": project_revision,
        "os": platform.system(),
        "architecture": platform.machine(),
        "python": platform.python_version(),
        "tools": {
            **runtime_tools,
            "font_bundle_sha256": font_bundle_sha256,
            "browser_version": run1.browser_version,
        },
        "artifacts": {
            name: evidence_binding(evidence_root, path)
            for name, path in sorted(runtime_artifacts.items())
        },
    }
    write_canonical_json(runtime_identity, runtime_value)
    runtime_binding = evidence_binding(evidence_root, runtime_identity)
    runtime_hash = sha256_file(runtime_identity)
    execution = output_dir / "execution.json"
    write_canonical_json(
        execution,
        {
            "schema_version": 1,
            "status": "PASS",
            "role": "candidate",
            "project_revision": project_revision,
            "evaluator_manifest_sha256": evaluator_hash,
            "corpus_manifest_sha256": corpus_hash,
            "network_isolation": "disabled",
            "source_count": len(source_set.sources),
            "unit_count": len(units),
            "external_requests": [],
            "determinism_runs": 2,
        },
    )
    execution_binding = evidence_binding(evidence_root, execution)
    upstream = output_dir / "upstream.json"
    common: dict[str, JsonValue] = {
        "schema_version": 1,
        "status": "READY",
        "role": "candidate",
        "format": source_set.document_format.value,
        "producer": "document2html-candidate",
        "runtime_sha256": runtime_hash,
        "runtime_identity": runtime_binding,
        "project_revision": project_revision,
        "contract_sha256": contract_hash,
        "corpus_manifest_sha256": corpus_hash,
        "evaluator_manifest_sha256": evaluator_hash,
        "oracle_lock_sha256": oracle_hash,
        "units": units,
        "files": files,
        "execution_log": execution_binding,
    }
    determinism = output_dir / "determinism.json"
    write_canonical_json(
        determinism,
        {
            "runs": [
                determinism_run_value(evidence_root, run1),
                determinism_run_value(evidence_root, run2),
            ]
        },
    )
    receipt = write_execution_receipt(
        evidence_root,
        output_dir,
        receipt_signer,
        runtime_artifacts["sandbox_public_key"],
        runtime_artifacts["openssl_binary"],
        oracle_lock_path,
        run_nonce=runtime_tools["run_nonce"],
        project_revision=project_revision,
        contract_sha256=contract_hash,
        corpus_sha256=corpus_hash,
        evaluator_sha256=evaluator_hash,
        oracle_lock_sha256=oracle_hash,
        runtime_identity=runtime_identity,
        execution_log=execution,
        determinism=determinism,
        runs=(run1, run2),
        runtime_artifacts=runtime_artifacts,
    )
    common["determinism_manifest"] = evidence_binding(
        evidence_root,
        determinism,
    )
    common["execution_receipt"] = evidence_binding(evidence_root, receipt)
    write_canonical_json(upstream, common)
    pending = output_dir / "manifest.pending.json"
    manifest: dict[str, JsonValue] = {
        "schema_version": 1,
        "status": "READY",
        "role": "candidate",
        "format": source_set.document_format.value,
        "producer": "document2html-candidate",
        "runtime_sha256": runtime_hash,
        "runtime_identity": runtime_binding,
        "contract_sha256": contract_hash,
        "corpus_manifest_sha256": corpus_hash,
        "evaluator_manifest_sha256": evaluator_hash,
        "oracle_lock_sha256": oracle_hash,
        "network_isolation": "disabled",
        "rendering": _rendering(source_set),
        "upstream_manifest": evidence_binding(evidence_root, upstream),
        "determinism_manifest": evidence_binding(evidence_root, determinism),
        "execution_receipt": evidence_binding(evidence_root, receipt),
        "units": units,
        "files": files,
    }
    write_canonical_json(pending, manifest)
    try:
        validate_capture_manifest(
            pending,
            "candidate",
            load_metric_spec(corpus_path),
            contract_hash,
            corpus_hash,
            evaluator_hash,
            oracle_hash,
            project_revision,
            evidence_root,
            oracle_lock_path,
        )
    except MetricError as error:
        pending.unlink(missing_ok=True)
        raise CandidateManifestError(
            "candidate manifest self-validation failed"
        ) from error
    capture = output_dir / "manifest.json"
    pending.replace(capture)
    return CandidateManifestPaths(
        capture,
        upstream,
        execution,
        runtime_identity,
        determinism,
    )


def _capture_records(
    root: Path,
    source_set: CandidateSourceSet,
    run: CandidateRun,
) -> tuple[list[dict[str, JsonValue]], list[dict[str, JsonValue]]]:
    units: list[dict[str, JsonValue]] = []
    files: list[dict[str, JsonValue]] = []
    for source_spec, source in zip(source_set.sources, run.sources, strict=True):
        files.append(
            {
                "source_id": source.source_id,
                "source_sha256": source.source_sha256,
                "html": evidence_binding(root, source.html),
            }
        )
        for unit_spec, unit in zip(source_spec.units, source.units, strict=True):
            units.append(
                {
                    "unit_id": unit.unit_id,
                    "source_id": source.source_id,
                    "source_sha256": source.source_sha256,
                    "ordinal": unit_spec.ordinal,
                    "png": evidence_binding(root, unit.png),
                    "inventory": evidence_binding(root, unit.inventory),
                }
            )
    return units, files


def _rendering(source_set: CandidateSourceSet) -> dict[str, JsonValue]:
    if source_set.document_format.value in {"ppt", "pptx"}:
        return {"dpi": None, "width": 960, "height": 540}
    return {"dpi": 144, "width": None, "height": None}
