import argparse
import hashlib
import json
import sys
from collections.abc import Sequence
from pathlib import Path

from evaluate.multiformat_evaluator_files import EVALUATOR_FILES
from evaluate.multiformat_portable_lock import portable_lock_template
from evaluate.multiformat_revision import current_project_revision
from evaluate.multiformat_scaffold_templates import (
    candidate_sandbox_attestation_template,
    office_lock_template,
)
from evaluate.multiformat_schema import (
    JsonValue,
    read_object,
    string_list,
    string_value,
)


class ScaffoldError(RuntimeError):
    pass


def scaffold_evidence(
    project_root: Path,
    contract_path: Path,
    output_dir: Path,
) -> None:
    try:
        project_root = project_root.resolve(strict=True)
        contract_path = contract_path.resolve(strict=True)
        contract = read_object(contract_path)
        formats = string_list(contract, "required_formats")
        corpus = _required_object(contract, "corpus")
        stratum_quotas = _required_object(contract, "stratum_quotas")
        metric_parameters = _required_object(contract, "metric_parameters")
        evaluator_lock = read_object(
            project_root / "evaluate" / "multiformat" / "evaluator-lock.v1.json"
        )
        evaluator_python = string_value(evaluator_lock, "python")
        unicode_version = string_value(evaluator_lock, "unicode_version")
        dependencies = _required_object(evaluator_lock, "dependencies")
        format_quotas = {
            document_format: _required_object(stratum_quotas, document_format)
            for document_format in formats
        }
        conformance_units = _required_int(corpus, "conformance_units")
        blind_files = _required_int(corpus, "blind_files")
        security_cases = _required_int(corpus, "security_cases")
        contract_sha256 = _sha256(contract_path)
        project_revision = current_project_revision(project_root)
        evaluator_files = [
            {
                "path": relative_path,
                "sha256": _sha256(project_root / relative_path),
            }
            for relative_path in EVALUATOR_FILES
        ]
    except (OSError, TypeError, ValueError) as error:
        raise ScaffoldError("scaffold input validation failed") from error

    _prepare_output(output_dir)
    evaluator_path = output_dir / "evidence" / "evaluator-manifest.json"
    evaluator_path.parent.mkdir(parents=True)
    evaluator_manifest = {
        "schema_version": 2,
        "contract_sha256": contract_sha256,
        "project_revision": project_revision,
        "python": evaluator_python,
        "unicode_version": unicode_version,
        "algorithm_parameters": metric_parameters,
        "dependencies": dependencies,
        "files": evaluator_files,
    }
    _write_json(evaluator_path, evaluator_manifest)
    evaluator_binding = _binding(output_dir, evaluator_path)

    for document_format in formats:
        corpus_path = output_dir / "corpora" / document_format / "manifest.json"
        metrics_path = output_dir / "metrics" / f"{document_format}.json"
        report_path = output_dir / "reports" / f"{document_format}.json"
        corpus_path.parent.mkdir(parents=True)
        metrics_path.parent.mkdir(parents=True, exist_ok=True)
        report_path.parent.mkdir(parents=True, exist_ok=True)
        _write_json(
            corpus_path,
            {
                "schema_version": 2,
                "status": "INCOMPLETE",
                "format": document_format,
                "contract_sha256": contract_sha256,
                "stratum_quotas": format_quotas[document_format],
                "tracks": {
                    "conformance": {
                        "expected_count": conformance_units,
                        "items": [],
                    },
                    "blind": {
                        "expected_count": blind_files,
                        "items": [],
                    },
                    "security": {
                        "expected_count": security_cases,
                        "items": [],
                    },
                },
            },
        )
        _write_json(
            metrics_path,
            {
                "schema_version": 2,
                "status": "INCOMPLETE",
                "format": document_format,
                "bindings": {},
                "conformance": {"units": []},
                "blind": {"files": []},
                "security": {"cases": []},
                "determinism": {"runs": []},
                "review": {"reviewers": []},
                "quality": {},
                "performance": {},
            },
        )
        _write_json(
            report_path,
            {
                "schema_version": 1,
                "status": "INCOMPLETE",
                "format": document_format,
                "contract_sha256": contract_sha256,
                "evaluator": evaluator_binding,
                "corpus_manifest": _binding(output_dir, corpus_path),
                "metrics_evidence": _binding(output_dir, metrics_path),
                "missing": [
                    "oracle_lock",
                    "conformance_evidence",
                    "blind_evidence",
                    "security_evidence",
                    "review_evidence",
                ],
            },
        )

    _write_json(
        output_dir / "oracle-lock.template.json",
        office_lock_template(),
    )
    _write_json(output_dir / "portable-lock.template.json", portable_lock_template())
    _write_json(
        output_dir / "office-input-manifest.json", {"schema_version": 1, "files": []}
    )
    _write_json(
        output_dir / "candidate-sandbox-attestation.template.json",
        candidate_sandbox_attestation_template(),
    )
    _write_json(
        output_dir / "font-bundle-manifest.template.json",
        {"schema_version": 1, "fonts": []},
    )


def _prepare_output(output_dir: Path) -> None:
    if output_dir.exists() and any(output_dir.iterdir()):
        raise ScaffoldError(f"output directory is not empty: {output_dir}")
    output_dir.mkdir(parents=True, exist_ok=True)


def _binding(root: Path, path: Path) -> dict[str, str]:
    return {
        "path": path.relative_to(root).as_posix(),
        "sha256": _sha256(path),
    }


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _write_json(path: Path, value: JsonValue) -> None:
    path.write_text(
        json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def _required_object(
    values: dict[str, JsonValue],
    field: str,
) -> dict[str, JsonValue]:
    value = values.get(field)
    if not isinstance(value, dict):
        raise ScaffoldError(f"{field} must be an object")
    return value


def _required_int(values: dict[str, JsonValue], field: str) -> int:
    value = values.get(field)
    if not isinstance(value, int) or isinstance(value, bool):
        raise ScaffoldError(f"{field} must be an integer")
    return value


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Create fail-closed seven-format evidence templates.",
    )
    parser.add_argument("--project-root", type=Path, default=Path.cwd())
    parser.add_argument(
        "--contract",
        type=Path,
        default=Path("evaluate/multiformat/contract.v1.json"),
    )
    parser.add_argument("--output-dir", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        scaffold_evidence(args.project_root, args.contract, args.output_dir)
    except ScaffoldError as error:
        sys.stderr.write(f"error: {error}\n")
        return 1
    sys.stdout.write(f"Scaffolded incomplete evidence at {args.output_dir}\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
