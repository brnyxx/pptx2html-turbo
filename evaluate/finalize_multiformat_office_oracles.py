from __future__ import annotations

import argparse
from collections import defaultdict
from pathlib import Path

from evaluate.multiformat_candidate_artifacts import evidence_binding
from evaluate.multiformat_corpus_types import CorpusError
from evaluate.multiformat_inventory import parse_inventory
from evaluate.multiformat_metric_links import load_metric_spec
from evaluate.multiformat_metric_types import MetricError
from evaluate.multiformat_office_oracle_batch import (
    OfficeOracleBatch,
    OfficeOracleBatchError,
    load_office_oracle_batch,
)
from evaluate.multiformat_office_oracle_batch_materialize import (
    materialize_office_oracle_batch,
)
from evaluate.multiformat_office_oracle_finalize_runtime import (
    OfficeOracleRuntimeBuildError,
    write_oracle_runtime_evidence,
)
from evaluate.multiformat_office_oracle_inventory import (
    OfficeOracleInventoryError,
    write_office_oracle_inventories,
)
from evaluate.multiformat_office_oracle_manifest import (
    write_office_oracle_manifests,
)
from evaluate.multiformat_office_oracle_receipt import (
    OfficeOracleReceiptError,
    write_office_oracle_receipt,
)
from evaluate.multiformat_schema import JsonValue, sha256_file, string_value
from evaluate.multiformat_strict_json import read_strict_object


class OfficeOracleFinalizeError(Exception):
    pass


def finalize_office_oracle(
    *,
    batch_manifest: Path,
    contract: Path,
    corpus_manifest: Path,
    evaluator_manifest: Path,
    oracle_lock: Path,
    output_dir: Path,
    receipt_signer: Path,
    public_key: Path,
    openssl: Path,
    project_revision: str,
    run_nonce: str,
) -> Path:
    try:
        root = _prepare_output(output_dir)
        source_batch = load_office_oracle_batch(batch_manifest)
        _validate_batch_lock(source_batch, oracle_lock, project_revision)
        spec = load_metric_spec(corpus_manifest)
        identities = spec.capture_identities()
        batch = materialize_office_oracle_batch(
            source_batch,
            root,
            {identity[0] for identity in identities.values()},
        )
        _validate_batch_lock(batch, oracle_lock, project_revision)
        units, generated_inventories = _capture_units(
            batch,
            identities,
            spec.document_format.value,
            root,
        )
        producer = (
            "locked-pdf-renderer"
            if spec.document_format.value == "pdf"
            else "windows-office-native"
        )
        runtime_identity, execution_log, runtime_artifacts = (
            write_oracle_runtime_evidence(
                root,
                batch,
                receipt_signer,
                public_key,
                openssl,
                oracle_lock,
                producer,
                project_revision,
                sha256_file(corpus_manifest),
                sha256_file(evaluator_manifest),
                run_nonce,
                len({str(unit["source_id"]) for unit in units}),
                len(units),
            )
        )
        receipt_dir = root / "receipt"
        receipt_dir.mkdir()
        receipt = write_office_oracle_receipt(
            root,
            receipt_dir,
            runtime_artifacts["receipt_signer_binary"],
            runtime_artifacts["office_oracle_public_key"],
            runtime_artifacts["openssl_binary"],
            oracle_lock,
            run_nonce=run_nonce,
            project_revision=project_revision,
            contract_sha256=sha256_file(contract),
            corpus_sha256=sha256_file(corpus_manifest),
            evaluator_sha256=sha256_file(evaluator_manifest),
            oracle_lock_sha256=sha256_file(oracle_lock),
            batch_manifest=batch.manifest,
            runtime_identity=runtime_identity,
            execution_log=execution_log,
            artifacts=sorted(
                {
                    *batch.artifacts,
                    *generated_inventories,
                    *runtime_artifacts.values(),
                },
                key=lambda item: item.as_posix(),
            ),
        )
        return write_office_oracle_manifests(
            root,
            document_format=spec.document_format.value,
            producer=producer,
            project_revision=project_revision,
            contract_sha256=sha256_file(contract),
            corpus_sha256=sha256_file(corpus_manifest),
            evaluator_sha256=sha256_file(evaluator_manifest),
            oracle_lock_sha256=sha256_file(oracle_lock),
            runtime_identity=runtime_identity,
            execution_log=execution_log,
            batch_manifest=batch.manifest,
            receipt=receipt,
            units=units,
        )
    except OfficeOracleFinalizeError:
        raise
    except (
        OfficeOracleBatchError,
        OfficeOracleReceiptError,
        OfficeOracleRuntimeBuildError,
        OfficeOracleInventoryError,
        CorpusError,
        MetricError,
        OSError,
        TypeError,
        ValueError,
    ) as error:
        raise OfficeOracleFinalizeError("office oracle finalization failed") from error


def _capture_units(
    batch: OfficeOracleBatch,
    identities: dict[str, tuple[str, str, int]],
    document_format: str,
    root: Path,
) -> tuple[list[dict[str, JsonValue]], list[Path]]:
    by_source: defaultdict[str, list[tuple[str, str, int]]] = defaultdict(list)
    for unit_id, identity in identities.items():
        by_source[identity[0]].append((unit_id, identity[1], identity[2]))
    if set(batch.files) != set(by_source):
        raise OfficeOracleFinalizeError("office oracle source set differs")
    result: list[dict[str, JsonValue]] = []
    generated: list[Path] = []
    for source_id, expected in sorted(by_source.items()):
        source = batch.files[source_id]
        ordered = sorted(expected, key=lambda item: item[2])
        if (
            source.source_sha256 != ordered[0][1]
            or source.document_format != document_format
            or len(source.units) != len(ordered)
        ):
            raise OfficeOracleFinalizeError("office oracle source identity differs")
        inventories = write_office_oracle_inventories(
            source,
            [unit_id for unit_id, _, _ in ordered],
            root / "inventories" / source_id,
        )
        generated.extend(inventories)
        for unit, inventory, (unit_id, source_hash, ordinal) in zip(
            source.units,
            inventories,
            ordered,
            strict=True,
        ):
            parsed = parse_inventory(inventory, unit_id)
            # Publishing an oracle whose cell attribution is incomplete would
            # bake unprovable evidence into the lock, so refuse it here.
            if parsed.unattributed_cells:
                raise OfficeOracleFinalizeError(
                    "office oracle inventory has unattributed cells"
                )
            result.append(
                {
                    "unit_id": unit_id,
                    "source_id": source_id,
                    "source_sha256": source_hash,
                    "ordinal": ordinal,
                    "png": evidence_binding(root, unit.png),
                    "inventory": evidence_binding(root, inventory),
                }
            )
    result.sort(key=lambda item: str(item["unit_id"]))
    return result, generated


def _prepare_output(output_dir: Path) -> Path:
    if output_dir.exists() and any(output_dir.iterdir()):
        raise OfficeOracleFinalizeError("office oracle output must be empty")
    output_dir.mkdir(parents=True, exist_ok=True)
    return output_dir.resolve(strict=True)


def _validate_batch_lock(
    batch: OfficeOracleBatch,
    oracle_lock: Path,
    project_revision: str,
) -> None:
    lock = read_strict_object(oracle_lock)
    if (
        batch.golden_set_revision != project_revision
        or batch.font_bundle_sha256 != string_value(lock, "font_bundle_sha256")
    ):
        raise OfficeOracleFinalizeError("office batch lock differs")


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Finalize a signed Windows Office oracle capture.",
    )
    parser.add_argument("--batch-manifest", type=Path, required=True)
    parser.add_argument("--contract", type=Path, required=True)
    parser.add_argument("--corpus-manifest", type=Path, required=True)
    parser.add_argument("--evaluator-manifest", type=Path, required=True)
    parser.add_argument("--oracle-lock", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--receipt-signer", type=Path, required=True)
    parser.add_argument("--public-key", type=Path, required=True)
    parser.add_argument("--openssl", type=Path, required=True)
    parser.add_argument("--project-revision", required=True)
    parser.add_argument("--run-nonce", required=True)
    arguments = parser.parse_args()
    try:
        finalize_office_oracle(
            batch_manifest=arguments.batch_manifest,
            contract=arguments.contract,
            corpus_manifest=arguments.corpus_manifest,
            evaluator_manifest=arguments.evaluator_manifest,
            oracle_lock=arguments.oracle_lock,
            output_dir=arguments.output_dir,
            receipt_signer=arguments.receipt_signer,
            public_key=arguments.public_key,
            openssl=arguments.openssl,
            project_revision=arguments.project_revision,
            run_nonce=arguments.run_nonce,
        )
    except OfficeOracleFinalizeError as error:
        parser.error(str(error))


if __name__ == "__main__":
    main()
