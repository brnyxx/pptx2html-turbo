from __future__ import annotations

import argparse
import json
import sys
from collections.abc import Sequence
from pathlib import Path

from evaluate.multiformat_command_evidence import (
    CommandEvidenceError,
    load_command_plan,
    run_performance_command,
    run_quality_commands,
    run_security_cases,
)
from evaluate.multiformat_metric_manifest import (
    MetricsAssemblyError,
    build_metrics_manifest,
    derive_metric_tracks,
    prepare_metric_context,
    publish_validated_metrics,
)
from evaluate.multiformat_metric_types import MetricError
from evaluate.multiformat_review_materialize import (
    ReviewMaterializeError,
    materialize_review_attestations,
)


def assemble_metrics(
    *,
    project_root: Path,
    contract_path: Path,
    corpus_path: Path,
    evaluator_path: Path,
    oracle_lock_path: Path,
    oracle_capture_path: Path,
    candidate_capture_path: Path,
    evidence_root: Path,
    commands_path: Path,
    review_paths: tuple[Path, ...],
    execution_output_dir: Path,
    output_path: Path,
    timeout_seconds: int = 120,
) -> None:
    evidence_root = evidence_root.resolve(strict=True)
    if timeout_seconds <= 0:
        raise MetricsAssemblyError("timeout must be positive")
    if execution_output_dir.exists() and any(execution_output_dir.iterdir()):
        raise MetricsAssemblyError("execution output directory is not empty")
    execution_output_dir.mkdir(parents=True, exist_ok=True)
    context = prepare_metric_context(
        project_root,
        contract_path,
        corpus_path,
        evaluator_path,
        oracle_lock_path,
        oracle_capture_path,
        candidate_capture_path,
        evidence_root,
    )
    plan = load_command_plan(commands_path)
    substitutions = {
        "project_revision": context.project_revision,
        "evaluator_hash": context.evaluator_hash,
        "corpus_hash": context.corpus_hash,
        "format": context.spec.document_format.value,
    }
    security = run_security_cases(
        plan,
        corpus_path,
        evidence_root,
        execution_output_dir,
        project_revision=context.project_revision,
        evaluator_hash=context.evaluator_hash,
        corpus_hash=context.corpus_hash,
        timeout_seconds=timeout_seconds,
    )
    quality = run_quality_commands(
        plan,
        evidence_root,
        execution_output_dir,
        bindings=substitutions,
        timeout_seconds=timeout_seconds,
    )
    performance = run_performance_command(
        plan,
        evidence_root,
        execution_output_dir,
        bindings=substitutions,
        timeout_seconds=timeout_seconds,
    )
    try:
        reviews, critical_defects = materialize_review_attestations(
            review_paths,
            context.spec.pair_ids(),
            context.oracle,
            context.candidate,
            context.evaluator_hash,
            context.corpus_hash,
            context.project_revision,
            evidence_root,
            execution_output_dir / "reviews",
        )
    except ReviewMaterializeError as error:
        raise MetricsAssemblyError(str(error)) from error
    conformance, blind = derive_metric_tracks(context, critical_defects)
    value = build_metrics_manifest(
        context,
        conformance,
        blind,
        security,
        reviews,
        quality,
        performance,
    )
    publish_validated_metrics(
        value,
        output_path,
        context,
        contract_path,
        corpus_path,
        evidence_root,
        oracle_lock_path,
    )


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Assemble raw capture and command evidence into metrics.",
    )
    parser.add_argument("--project-root", type=Path, default=Path("."))
    parser.add_argument("--contract", type=Path, required=True)
    parser.add_argument("--corpus-manifest", type=Path, required=True)
    parser.add_argument("--evaluator-manifest", type=Path, required=True)
    parser.add_argument("--oracle-lock", type=Path, required=True)
    parser.add_argument("--oracle-capture", type=Path, required=True)
    parser.add_argument("--candidate-capture", type=Path, required=True)
    parser.add_argument("--evidence-root", type=Path, required=True)
    parser.add_argument("--commands", type=Path, required=True)
    parser.add_argument(
        "--review-decisions",
        type=Path,
        action="append",
        required=True,
    )
    parser.add_argument("--execution-output-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--timeout-seconds", type=int, default=120)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        assemble_metrics(
            project_root=args.project_root,
            contract_path=args.contract,
            corpus_path=args.corpus_manifest,
            evaluator_path=args.evaluator_manifest,
            oracle_lock_path=args.oracle_lock,
            oracle_capture_path=args.oracle_capture,
            candidate_capture_path=args.candidate_capture,
            evidence_root=args.evidence_root,
            commands_path=args.commands,
            review_paths=tuple(args.review_decisions),
            execution_output_dir=args.execution_output_dir,
            output_path=args.output,
            timeout_seconds=args.timeout_seconds,
        )
    except (
        CommandEvidenceError,
        MetricsAssemblyError,
        MetricError,
        OSError,
        TypeError,
        ValueError,
    ) as error:
        sys.stdout.write(json.dumps({"status": "FAIL", "reason": str(error)}) + "\n")
        return 1
    sys.stdout.write(
        json.dumps(
            {"status": "READY", "metrics_evidence": args.output.as_posix()},
            sort_keys=True,
        )
        + "\n"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
