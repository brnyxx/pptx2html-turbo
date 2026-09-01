from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Sequence

from evaluate.multiformat_evaluator_manifest import validate_evaluator_manifest
from evaluate.multiformat_evidence import oracle_lock_ready, resolve_evidence_path
from evaluate.multiformat_metric_types import MetricError
from evaluate.multiformat_metrics import validate_metrics_evidence
from evaluate.multiformat_report import build_report
from evaluate.multiformat_schema import JsonValue, sha256_file
from evaluate.multiformat_strict_json import read_strict_object


def assemble_report(
    project_root: Path,
    contract_path: Path,
    oracle_lock_path: Path,
    evaluator_manifest_path: Path,
    corpus_manifest_path: Path,
    metrics_evidence_path: Path,
    evidence_root: Path,
) -> dict[str, JsonValue]:
    project_root = project_root.resolve(strict=True)
    evidence_root = evidence_root.resolve(strict=True)
    if not oracle_lock_ready(oracle_lock_path, evidence_root):
        raise MetricError("oracle_lock", oracle_lock_path.as_posix())
    evaluator_hash = validate_evaluator_manifest(
        project_root,
        contract_path,
        evaluator_manifest_path,
    )
    oracle_hash = sha256_file(oracle_lock_path)
    summary = validate_metrics_evidence(
        contract_path,
        corpus_manifest_path,
        metrics_evidence_path,
        evaluator_hash,
        oracle_hash,
        evidence_root,
        oracle_lock_path,
    )
    return build_report(
        summary,
        sha256_file(contract_path),
        oracle_hash,
        _binding(evidence_root, evaluator_manifest_path),
        _binding(evidence_root, corpus_manifest_path),
        _binding(evidence_root, metrics_evidence_path),
    )


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Assemble one canonical multi-format acceptance report.",
    )
    parser.add_argument("--project-root", type=Path, default=Path("."))
    parser.add_argument(
        "--contract",
        type=Path,
        default=Path("evaluate/multiformat/contract.v1.json"),
    )
    parser.add_argument("--oracle-lock", type=Path, required=True)
    parser.add_argument("--evaluator-manifest", type=Path, required=True)
    parser.add_argument("--corpus-manifest", type=Path, required=True)
    parser.add_argument("--metrics-evidence", type=Path, required=True)
    parser.add_argument("--evidence-root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    if _is_incomplete(args.corpus_manifest) or _is_incomplete(args.metrics_evidence):
        sys.stdout.write(
            json.dumps(
                {"status": "INCOMPLETE"},
                ensure_ascii=True,
                sort_keys=True,
            )
            + "\n"
        )
        return 2
    try:
        report = assemble_report(
            args.project_root,
            args.contract,
            args.oracle_lock,
            args.evaluator_manifest,
            args.corpus_manifest,
            args.metrics_evidence,
            args.evidence_root,
        )
        _write_report(args.output, report)
    except (MetricError, OSError, ValueError) as error:
        sys.stdout.write(
            json.dumps(
                {"status": "FAIL", "reason": str(error)},
                ensure_ascii=True,
                sort_keys=True,
            )
            + "\n"
        )
        return 1
    sys.stdout.write(json.dumps(report, ensure_ascii=True, sort_keys=True) + "\n")
    return 0


def _binding(root: Path, path: Path) -> dict[str, JsonValue]:
    relative = path.resolve(strict=True).relative_to(root).as_posix()
    resolved = resolve_evidence_path(root, relative)
    return {"path": relative, "sha256": sha256_file(resolved)}


def _write_report(path: Path, report: dict[str, JsonValue]) -> None:
    if path.exists():
        existing = read_strict_object(path)
        if existing.get("status") != "INCOMPLETE":
            raise MetricError("report.output", "refusing to replace READY evidence")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(report, ensure_ascii=True, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def _is_incomplete(path: Path) -> bool:
    try:
        return read_strict_object(path).get("status") == "INCOMPLETE"
    except (OSError, ValueError):
        return False


if __name__ == "__main__":
    raise SystemExit(main())
