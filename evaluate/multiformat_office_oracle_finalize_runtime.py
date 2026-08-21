from __future__ import annotations

import platform
import shutil
import subprocess
from pathlib import Path

from evaluate.multiformat_candidate_artifacts import (
    evidence_binding,
    write_canonical_json,
)
from evaluate.multiformat_office_oracle_batch import OfficeOracleBatch
from evaluate.multiformat_schema import (
    JsonValue,
    object_value,
    sha256_file,
    string_value,
)
from evaluate.multiformat_strict_json import read_strict_object
from evaluate.multiformat_subprocess import clean_subprocess_environment


class OfficeOracleRuntimeBuildError(Exception):
    pass


def write_oracle_runtime_evidence(
    root: Path,
    batch: OfficeOracleBatch,
    receipt_signer: Path,
    public_key: Path,
    openssl: Path,
    oracle_lock: Path,
    producer: str,
    project_revision: str,
    corpus_hash: str,
    evaluator_hash: str,
    run_nonce: str,
    source_count: int,
    unit_count: int,
) -> tuple[Path, Path, dict[str, Path]]:
    runtime_root = root / "runtime"
    runtime_root.mkdir()
    artifacts = {
        "receipt_signer_binary": _copy_runtime(
            receipt_signer,
            runtime_root / f"receipt-signer{receipt_signer.suffix}",
        ),
        "office_oracle_public_key": _copy_runtime(
            public_key,
            runtime_root / "office-oracle-public.pem",
        ),
        "openssl_binary": _copy_runtime(
            openssl,
            runtime_root / f"openssl{openssl.suffix}",
        ),
    }
    signer_version = _bounded_version(artifacts["receipt_signer_binary"])
    verifier = object_value(
        read_strict_object(oracle_lock),
        "office_oracle_verifier",
    )
    runtime = runtime_root / "identity.json"
    runtime_values: dict[str, JsonValue] = {
        "schema_version": 1,
        "role": "oracle",
        "producer": producer,
        "project_revision": project_revision,
        "os": string_value(batch.runtime, "windows"),
        "architecture": string_value(batch.runtime, "architecture"),
        "python": platform.python_version(),
        "tools": {
            "office_channel": string_value(batch.runtime, "office_channel"),
            "word_version": string_value(batch.runtime, "word"),
            "excel_version": string_value(batch.runtime, "excel"),
            "powerpoint_version": string_value(batch.runtime, "powerpoint"),
            "pdf_primary_version": string_value(batch.runtime, "pdf_primary"),
            "pdf_secondary_version": string_value(batch.runtime, "pdf_secondary"),
            "pdf_text_version": string_value(batch.runtime, "pdf_text"),
            "office_oracle_public_key_sha256": sha256_file(
                artifacts["office_oracle_public_key"]
            ),
            "openssl_sha256": sha256_file(artifacts["openssl_binary"]),
            "receipt_signer_sha256": sha256_file(artifacts["receipt_signer_binary"]),
            "receipt_signer_version": signer_version,
            "office_oracle_verifier_id": string_value(verifier, "verifier_id"),
            "run_nonce": run_nonce,
        },
        "artifacts": {
            name: evidence_binding(root, path) for name, path in artifacts.items()
        },
    }
    write_canonical_json(runtime, runtime_values)
    execution = root / "execution.json"
    write_canonical_json(
        execution,
        {
            "schema_version": 1,
            "status": "PASS",
            "role": "oracle",
            "project_revision": project_revision,
            "evaluator_manifest_sha256": evaluator_hash,
            "corpus_manifest_sha256": corpus_hash,
            "network_isolation": "disabled",
            "source_count": source_count,
            "unit_count": unit_count,
            "external_requests": [],
            "determinism_runs": 1,
        },
    )
    return runtime, execution, artifacts


def _copy_runtime(source: Path, destination: Path) -> Path:
    shutil.copy2(source.resolve(strict=True), destination)
    return destination


def _bounded_version(executable: Path) -> str:
    result = subprocess.run(
        [executable.as_posix(), "--version"],
        check=False,
        capture_output=True,
        env=clean_subprocess_environment(),
        timeout=15,
    )
    if result.returncode != 0 or len(result.stdout) > 1024 * 1024:
        raise OfficeOracleRuntimeBuildError("oracle signer version failed")
    value = result.stdout.decode(errors="strict").strip()
    if not value:
        raise OfficeOracleRuntimeBuildError("oracle signer version is empty")
    return value
