from __future__ import annotations

import importlib.metadata
from dataclasses import dataclass
from pathlib import Path

from evaluate.multiformat_candidate_attestation import (
    attestation_scope_sha256,
    verify_signed_attestation,
)
from evaluate.multiformat_candidate_sources import (
    CandidateSourceSet,
    load_candidate_sources,
)
from evaluate.multiformat_candidate_types import (
    CandidateCaptureError,
    CandidateRuntimePaths,
)
from evaluate.multiformat_corpus_types import CorpusError
from evaluate.multiformat_evaluator_manifest import validate_evaluator_manifest
from evaluate.multiformat_evidence import oracle_lock_ready
from evaluate.multiformat_candidate_fonts import prepare_font_environment
from evaluate.multiformat_candidate_runtime_lock import (
    require_browser_lock,
    require_clean_worktree as assert_clean_worktree,
    validate_candidate_runtime,
)
from evaluate.multiformat_metric_types import MetricError
from evaluate.multiformat_revision import current_project_revision
from evaluate.multiformat_schema import (
    object_value,
    sha256_file,
    sha256_value,
    string_value,
)
from evaluate.multiformat_strict_json import read_strict_object


class CandidatePreflightError(CandidateCaptureError):
    pass


@dataclass(frozen=True, slots=True)
class CandidatePreflight:
    source_set: CandidateSourceSet
    runtime: CandidateRuntimePaths
    project_revision: str
    runtime_tools: dict[str, str]
    runtime_artifacts: dict[str, Path]
    font_bundle_sha256: str


def preflight_candidate_capture(
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
    timeout_seconds: int,
    require_clean_worktree: bool,
    require_release_binary: bool,
) -> CandidatePreflight:
    try:
        project_root = project_root.resolve(strict=True)
        evidence_root = evidence_root.resolve(strict=True)
        if project_root.is_relative_to(evidence_root):
            raise CandidatePreflightError(
                "evidence root cannot contain the project root"
            )
        if require_clean_worktree:
            assert_clean_worktree(project_root, evidence_root)
        revision = current_project_revision(project_root)
        if output_dir.exists() and any(output_dir.iterdir()):
            raise CandidatePreflightError(
                f"candidate output is not empty: {output_dir}"
            )
        if output_dir.is_symlink():
            raise CandidatePreflightError("candidate output cannot be a symlink")
        output_parent = output_dir.parent.resolve(strict=True)
        if not output_parent.is_relative_to(evidence_root):
            raise CandidatePreflightError("candidate output escapes evidence root")
        if not oracle_lock_ready(oracle_lock_path):
            raise CandidatePreflightError("oracle lock is not ready")
        validate_evaluator_manifest(project_root, contract_path, evaluator_path)
        source_set = load_candidate_sources(contract_path, corpus_path)
        if (
            require_release_binary
            and converter.resolve(strict=True).parent.name != "release"
        ):
            raise CandidatePreflightError(
                "candidate capture requires a release converter"
            )
        lock = read_strict_object(oracle_lock_path)
        browser = object_value(lock, "browser")
        browser_version = string_value(browser, "chromium")
        executable = chromium.resolve(strict=True)
        if sha256_file(executable) != sha256_value(browser, "executable_sha256"):
            raise CandidatePreflightError("Chromium executable hash mismatch")
        playwright_version = importlib.metadata.version("playwright")
        if playwright_version != string_value(browser, "playwright"):
            raise CandidatePreflightError("Playwright version mismatch")
        require_browser_lock(browser)
        font_environment = prepare_font_environment(
            font_bundle,
            output_parent / ".candidate-font-runtime",
        )
        if font_environment.manifest_sha256 != sha256_value(
            lock,
            "font_bundle_sha256",
        ):
            raise CandidatePreflightError("font bundle hash mismatch")
        if font_environment.environment_sha256 != sha256_value(
            browser,
            "font_environment_sha256",
        ):
            raise CandidatePreflightError("font environment hash mismatch")
        runtime = CandidateRuntimePaths(
            converter.resolve(strict=True),
            soffice.resolve(strict=True),
            pdftohtml.resolve(strict=True),
            pdfinfo.resolve(strict=True),
            executable,
            receipt_signer.resolve(strict=True),
            font_environment.config_path,
            browser_version,
            timeout_seconds,
        )
        candidate_runtime = object_value(lock, "candidate_runtime")
        versions = validate_candidate_runtime(
            candidate_runtime,
            runtime,
            revision,
        )
        verified_attestation = verify_signed_attestation(
            sandbox_attestation,
            sandbox_public_key,
            openssl,
            oracle_lock_path,
            project_revision=revision,
            scope_sha256=attestation_scope_sha256(
                contract_path,
                corpus_path,
                evaluator_path,
                oracle_lock_path,
            ),
        )
        if (
            verified_attestation.font_environment_sha256
            != font_environment.environment_sha256
        ):
            raise CandidatePreflightError("signed font environment mismatch")
        for evidence_path in [
            sandbox_attestation,
            sandbox_public_key,
            font_bundle,
            font_environment.config_path,
        ]:
            if not evidence_path.resolve(strict=True).is_relative_to(evidence_root):
                raise CandidatePreflightError(
                    f"runtime evidence escapes evidence root: {evidence_path}"
                )
        runtime_tools = {
            "converter_sha256": sha256_file(runtime.converter),
            "soffice_sha256": sha256_file(runtime.soffice),
            "pdftohtml_sha256": sha256_file(runtime.pdftohtml),
            "pdfinfo_sha256": sha256_file(runtime.pdfinfo),
            "receipt_signer_sha256": sha256_file(runtime.receipt_signer),
            "chromium_sha256": sha256_file(runtime.chromium),
            "playwright": playwright_version,
            "sandbox_attestation_sha256": sha256_file(sandbox_attestation),
            "sandbox_public_key_sha256": sha256_file(sandbox_public_key),
            "openssl_sha256": sha256_file(openssl),
            "font_environment_sha256": font_environment.environment_sha256,
            "font_config_sha256": sha256_file(font_environment.config_path),
            "sandbox_verifier_id": verified_attestation.verifier_id,
            "run_nonce": verified_attestation.run_nonce,
            "build_revision": revision,
            **versions,
        }
        return CandidatePreflight(
            source_set,
            runtime,
            revision,
            runtime_tools,
            {
                "sandbox_attestation": sandbox_attestation.resolve(strict=True),
                "sandbox_public_key": sandbox_public_key.resolve(strict=True),
                "font_bundle": font_bundle.resolve(strict=True),
                "font_config": font_environment.config_path.resolve(strict=True),
                "converter_binary": runtime.converter,
                "soffice_binary": runtime.soffice,
                "pdftohtml_binary": runtime.pdftohtml,
                "pdfinfo_binary": runtime.pdfinfo,
                "chromium_binary": runtime.chromium,
                "openssl_binary": openssl.resolve(strict=True),
                "receipt_signer_binary": runtime.receipt_signer,
            },
            font_environment.manifest_sha256,
        )
    except CandidatePreflightError:
        raise
    except (
        MetricError,
        CorpusError,
        CandidateCaptureError,
        OSError,
        TypeError,
        ValueError,
        importlib.metadata.PackageNotFoundError,
    ) as error:
        raise CandidatePreflightError(str(error)) from error
