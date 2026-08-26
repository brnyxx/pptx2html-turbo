from __future__ import annotations

import argparse
import json
import shutil
import sys
from collections.abc import Sequence
from pathlib import Path

from evaluate.multiformat_candidate_capture import materialize_candidate_runtime
from evaluate.multiformat_candidate_preflight import preflight_candidate_capture
from evaluate.multiformat_candidate_security import (
    CandidateSecurityError,
    CandidateSecuritySource,
    execute_candidate_security_case,
    load_candidate_security_sources,
)
from evaluate.multiformat_candidate_types import CandidateCaptureError
from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_metric_types import MetricError
from evaluate.multiformat_schema import sha256_file


def load_exact_security_source(
    contract_path: Path,
    corpus_path: Path,
    source_id: str,
    declared_path: Path,
) -> tuple[DocumentFormat, CandidateSecuritySource]:
    document_format, sources = load_candidate_security_sources(
        contract_path, corpus_path
    )
    matching = tuple(source for source in sources if source.source_id == source_id)
    if len(matching) != 1:
        raise CandidateSecurityError(f"unknown security source ID: {source_id}")
    source = matching[0]
    try:
        actual_path = declared_path.resolve(strict=True)
    except OSError as error:
        raise CandidateSecurityError(
            "declared security source is unavailable"
        ) from error
    if actual_path != source.path.resolve(strict=True):
        raise CandidateSecurityError(f"security source path differs: {source_id}")
    return document_format, source


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Execute one locked candidate security corpus case."
    )
    parser.add_argument("--project-root", type=Path, required=True)
    parser.add_argument("--contract", type=Path, required=True)
    parser.add_argument("--corpus-manifest", type=Path, required=True)
    parser.add_argument("--evaluator-manifest", type=Path, required=True)
    parser.add_argument("--oracle-lock", type=Path, required=True)
    parser.add_argument("--evidence-root", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--source-id", required=True)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--converter", type=Path, required=True)
    parser.add_argument("--soffice", type=Path, required=True)
    parser.add_argument("--pdftohtml", type=Path, required=True)
    parser.add_argument("--pdfinfo", type=Path, required=True)
    parser.add_argument("--chromium", type=Path, required=True)
    parser.add_argument("--font-bundle", type=Path, required=True)
    parser.add_argument("--sandbox-attestation", type=Path, required=True)
    parser.add_argument("--sandbox-public-key", type=Path, required=True)
    parser.add_argument("--openssl", type=Path, required=True)
    parser.add_argument("--receipt-signer", type=Path, required=True)
    parser.add_argument("--timeout-seconds", type=int, default=120)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    output_created = False
    try:
        evidence_root = args.evidence_root.resolve(strict=True)
        paths = (
            args.contract,
            args.corpus_manifest,
            args.evaluator_manifest,
            args.oracle_lock,
        )
        before = tuple(sha256_file(path) for path in paths)
        preflight = preflight_candidate_capture(
            args.project_root,
            args.contract,
            args.corpus_manifest,
            args.evaluator_manifest,
            args.oracle_lock,
            evidence_root,
            args.output_dir,
            converter=args.converter,
            soffice=args.soffice,
            pdftohtml=args.pdftohtml,
            pdfinfo=args.pdfinfo,
            chromium=args.chromium,
            font_bundle=args.font_bundle,
            sandbox_attestation=args.sandbox_attestation,
            sandbox_public_key=args.sandbox_public_key,
            openssl=args.openssl,
            receipt_signer=args.receipt_signer,
            timeout_seconds=args.timeout_seconds,
            require_clean_worktree=True,
            require_release_binary=True,
        )
        if not preflight.runtime_profile.portable:
            raise CandidateSecurityError("security command requires a portable runtime")
        document_format, source = load_exact_security_source(
            args.contract, args.corpus_manifest, args.source_id, args.source
        )
        args.output_dir.mkdir(parents=True, exist_ok=False)
        output_created = True
        runtime, _artifacts = materialize_candidate_runtime(
            preflight, evidence_root, args.output_dir
        )
        result = execute_candidate_security_case(
            source, document_format, args.output_dir / "case", runtime
        )
        if tuple(sha256_file(path) for path in paths) != before:
            raise CandidateSecurityError(
                "locked security inputs changed during execution"
            )
    except (
        CandidateCaptureError,
        MetricError,
        OSError,
        TypeError,
        ValueError,
    ) as error:
        if output_created:
            shutil.rmtree(args.output_dir)
        sys.stderr.write(f"security-case: {error}\n")
        return 1
    sys.stdout.write(json.dumps(result.command_evidence(), sort_keys=True) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
