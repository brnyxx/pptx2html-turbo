from __future__ import annotations

from pathlib import Path

from evaluate.multiformat_candidate_artifacts import materialize_runtime_artifacts
from evaluate.multiformat_candidate_manifest import write_candidate_manifests
from evaluate.multiformat_candidate_preflight import preflight_candidate_capture
from evaluate.multiformat_candidate_preflight_types import CandidatePreflight
from evaluate.multiformat_candidate_run import capture_clean_run
from evaluate.multiformat_candidate_runtime_lock import (
    validate_candidate_runtime,
)
from evaluate.multiformat_candidate_sandbox import require_active_sandbox
from evaluate.multiformat_candidate_security import capture_candidate_security
from evaluate.multiformat_candidate_sources import (
    CandidateSourceSet,
    load_candidate_sources,
)
from evaluate.multiformat_candidate_types import (
    CandidateCaptureError,
    CandidateManifestPaths,
    CandidateRuntimePaths,
    RuntimeArtifactSnapshots,
)
from evaluate.multiformat_schema import sha256_file


def materialize_candidate_runtime(
    preflight: CandidatePreflight,
    evidence_root: Path,
    output_dir: Path,
) -> tuple[CandidateRuntimePaths, RuntimeArtifactSnapshots]:
    runtime_artifacts = materialize_runtime_artifacts(
        preflight.runtime_artifacts,
        evidence_root,
        output_dir / "runtime-inputs",
    )
    runtime = CandidateRuntimePaths(
        runtime_artifacts["converter_binary"],
        runtime_artifacts["soffice_binary"],
        runtime_artifacts["pdftohtml_binary"],
        runtime_artifacts["pdfinfo_binary"],
        runtime_artifacts["chromium_binary"],
        runtime_artifacts["receipt_signer_binary"],
        runtime_artifacts["font_config"],
        preflight.runtime.browser_version,
        preflight.runtime.timeout_seconds,
    )
    validate_candidate_runtime(
        preflight.runtime_profile.candidate_runtime_lock,
        runtime,
        preflight.project_revision,
    )
    return runtime, runtime_artifacts


def capture_candidate_evidence(
    project_root: Path,
    contract_path: Path,
    corpus_path: Path,
    evaluator_path: Path,
    oracle_lock_path: Path,
    evidence_root: Path,
    output_dir: Path,
    *,
    converter: Path,
    soffice: Path,
    pdftohtml: Path,
    pdfinfo: Path,
    chromium: Path,
    font_bundle: Path,
    sandbox_attestation: Path,
    sandbox_public_key: Path,
    openssl: Path,
    receipt_signer: Path,
    timeout_seconds: int = 120,
    require_clean_worktree: bool = True,
    require_release_binary: bool = True,
) -> CandidateManifestPaths:
    evidence_root = evidence_root.resolve(strict=True)
    output_dir = output_dir.parent.resolve(strict=True) / output_dir.name
    preflight = preflight_candidate_capture(
        project_root,
        contract_path,
        corpus_path,
        evaluator_path,
        oracle_lock_path,
        evidence_root,
        output_dir,
        converter=converter,
        soffice=soffice,
        pdftohtml=pdftohtml,
        pdfinfo=pdfinfo,
        chromium=chromium,
        font_bundle=font_bundle,
        sandbox_attestation=sandbox_attestation,
        sandbox_public_key=sandbox_public_key,
        openssl=openssl,
        receipt_signer=receipt_signer,
        timeout_seconds=timeout_seconds,
        require_clean_worktree=require_clean_worktree,
        require_release_binary=require_release_binary,
    )
    if preflight.runtime_profile.portable:
        if preflight.sandbox is None:
            raise CandidateCaptureError("candidate sandbox attestation is missing")
        require_active_sandbox(preflight.sandbox)
    output_dir.mkdir(parents=True, exist_ok=True)
    runtime, runtime_artifacts = materialize_candidate_runtime(
        preflight, evidence_root, output_dir
    )
    runtime_tools = {
        **preflight.runtime_tools,
        "font_config_sha256": sha256_file(runtime_artifacts["font_config"]),
        "runtime_package_sha256": sha256_file(
            runtime_artifacts["runtime_package_manifest"]
        ),
    }
    runtime_artifacts.revalidate()
    run1 = capture_clean_run(
        1,
        preflight.source_set,
        output_dir / "staging-run-1",
        evidence_root,
        runtime,
    )
    runtime_artifacts.revalidate()
    _revalidate_sources(
        contract_path,
        corpus_path,
        preflight.source_set,
    )
    runtime_artifacts.revalidate()
    run2 = capture_clean_run(
        2,
        preflight.source_set,
        output_dir / "staging-run-2",
        evidence_root,
        runtime,
    )
    runtime_artifacts.revalidate()
    _revalidate_sources(
        contract_path,
        corpus_path,
        preflight.source_set,
    )
    security_artifacts: tuple[Path, ...] = ()
    if preflight.runtime_profile.portable:
        runtime_artifacts.revalidate()
        security_artifacts = capture_candidate_security(
            contract_path,
            corpus_path,
            evaluator_path,
            output_dir / "security",
            runtime,
            preflight.project_revision,
        )
        runtime_artifacts.revalidate()
    manifests = write_candidate_manifests(
        evidence_root,
        output_dir / "published",
        preflight.source_set,
        run1,
        run2,
        contract_path,
        corpus_path,
        evaluator_path,
        oracle_lock_path,
        project_revision=preflight.project_revision,
        runtime_tools=runtime_tools,
        runtime_artifacts=runtime_artifacts,
        runtime_snapshots=runtime_artifacts,
        receipt_signer=runtime.receipt_signer,
        font_bundle_sha256=preflight.font_bundle_sha256,
        runtime_profile=preflight.runtime_profile,
        security_artifacts=security_artifacts,
    )
    runtime_artifacts.revalidate()
    return manifests


def _revalidate_sources(
    contract_path: Path,
    corpus_path: Path,
    expected: CandidateSourceSet,
) -> None:
    if load_candidate_sources(contract_path, corpus_path) != expected:
        raise CandidateCaptureError("candidate corpus changed between clean runs")
