from __future__ import annotations

from pathlib import Path

from evaluate.jcs import canonicalize
from evaluate.multiformat_corpus import validate_corpus_manifest
from evaluate.multiformat_corpus_types import CorpusError, DocumentFormat
from evaluate.multiformat_ready_assembly_manifest import build_assembly_manifest
from evaluate.multiformat_ready_assembly_types import (
    ReadyAssemblyError,
    ReadyAssemblyFailure,
    ReadyAssemblyInputs,
    ReadyAssemblySummary,
    ReadyValidationInputs,
)
from evaluate.multiformat_ready_copy import copy_stable_source
from evaluate.multiformat_ready_inputs import load_ready_inputs
from evaluate.multiformat_ready_manifest import build_format_manifest
from evaluate.multiformat_ready_tree import tree_identity
from evaluate.multiformat_ready_types import (
    ReadyBlind,
    ReadyConformance,
    ReadyInputError,
    ReadyInputPaths,
    ReadySecurity,
    ReadySource,
    ReadySourceSet,
)
from evaluate.multiformat_schema import JsonValue, sha256_file
from evaluate.multiformat_snapshot_publish import SnapshotPublishError, publish_snapshot


def assemble_ready_corpora(inputs: ReadyAssemblyInputs) -> ReadyAssemblySummary:
    """Build, independently validate, and atomically publish seven corpora."""
    try:
        source_set = load_ready_inputs(inputs.sources)
    except ReadyInputError as error:
        raise ReadyAssemblyError(
            ReadyAssemblyFailure.INPUT_INVALID, error.detail
        ) from error
    summaries: list[ReadyAssemblySummary] = []

    def writer(staging: Path) -> None:
        _write_candidate(inputs.sources, source_set, staging)
        from evaluate.multiformat_ready_validation import validate_ready_corpora

        summaries.append(
            validate_ready_corpora(ReadyValidationInputs(inputs.sources, staging))
        )

    try:
        publish_snapshot(
            inputs.output_dir,
            writer,
            lock_namespace="ready-corpora",
        )
    except ReadyAssemblyError:
        raise
    except SnapshotPublishError as error:
        raise ReadyAssemblyError(
            ReadyAssemblyFailure.PUBLICATION_FAILED, error.failure.value
        ) from error
    except (CorpusError, OSError, TypeError, ValueError) as error:
        raise ReadyAssemblyError(
            ReadyAssemblyFailure.VALIDATION_FAILED, str(error)
        ) from error
    return summaries[0]


def _write_candidate(
    paths: ReadyInputPaths,
    source_set: ReadySourceSet,
    staging: Path,
) -> None:
    copy_stable_source(
        paths.plan,
        staging / "conformance-plan.json",
        sha256_file(paths.plan),
        "conformance-plan.json",
    )
    inventory = paths.native_inventory_root / "native-unit-inventory.json"
    copy_stable_source(
        inventory,
        staging / "native-unit-inventory.json",
        sha256_file(inventory),
        "native-unit-inventory.json",
    )
    _copy_sources(source_set, staging)
    contract_digest = sha256_file(paths.contract)
    for document_format in DocumentFormat:
        manifest = build_format_manifest(contract_digest, document_format, source_set)
        destination = staging / "corpora" / document_format.value / "manifest.json"
        _write_json(destination, manifest)
        _ = validate_corpus_manifest(paths.contract, destination)
    identity = tree_identity(staging)
    if identity.files != 1484:
        raise ReadyAssemblyError(
            ReadyAssemblyFailure.MANIFEST_INVALID,
            f"tree file count: {identity.files}",
        )
    _write_json(
        staging / "assembly-manifest.json",
        build_assembly_manifest(paths, staging, source_set.supports, identity),
    )


def _copy_sources(source_set: ReadySourceSet, root: Path) -> None:
    for source in source_set.sources:
        relative = _source_relative_path(source)
        copy_stable_source(
            source.source_path,
            root / relative,
            source.source_sha256,
            relative,
        )
    for support in source_set.supports:
        relative = (
            f"corpora/{support.owner_format.value}/sources/support/{support.filename}"
        )
        copy_stable_source(
            support.source_path,
            root / relative,
            support.source_sha256,
            relative,
        )


def _source_relative_path(source: ReadySource) -> str:
    tracks: dict[type[ReadyConformance | ReadyBlind | ReadySecurity], str] = {
        ReadyConformance: "conformance",
        ReadyBlind: "blind",
        ReadySecurity: "security",
    }
    track = tracks[type(source.details)]
    return (
        f"corpora/{source.document_format.value}/sources/{track}/"
        f"{source.source_id}.{source.document_format.value}"
    )


def _write_json(path: Path, value: JsonValue) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    _ = path.write_bytes(canonicalize(value) + b"\n")
