from __future__ import annotations

from pathlib import Path
from typing import cast

from evaluate.multiformat_corpus_source_fs import stable_source_descriptor
from evaluate.multiformat_corpus_types import CorpusError, DocumentFormat
from evaluate.multiformat_ready_assembly_types import (
    ReadyAssemblyError,
    ReadyAssemblyFailure,
    ReadyAssemblySummary,
    ReadyValidationError,
    ReadyValidationInputs,
)
from evaluate.multiformat_ready_inputs import load_ready_inputs
from evaluate.multiformat_ready_tree import tree_identity
from evaluate.multiformat_ready_tree_fs import scan_tree
from evaluate.multiformat_ready_types import (
    ReadyInputError,
    ReadySourceSet,
    ReadySupport,
)
from evaluate.multiformat_ready_validation_relations import validate_support_relations
from evaluate.multiformat_ready_validation_schema import read_root_manifest
from evaluate.multiformat_ready_validation_sources import (
    validate_copied_inputs,
    validate_corpora,
)
from evaluate.multiformat_schema import JsonValue, sha256_file

_UPSTREAM_PATHS = (
    ("docx-conformance", "docx_conformance"),
    ("legacy-binary-config", "legacy_binary_config"),
    ("legacy-binary-manifest", "legacy_binary_manifest"),
    ("legacy-conformance", "legacy_conformance"),
    ("pdf-conformance", "pdf_conformance"),
    ("pptx-conformance", "pptx_conformance"),
    ("public-config", "public_config"),
    ("public-pool-manifest", "public_pool_manifest"),
    ("security-manifest", "security_manifest"),
    ("xlsx-conformance", "xlsx_conformance"),
)


def validate_ready_corpora(inputs: ReadyValidationInputs) -> ReadyAssemblySummary:
    """Independently validate a complete immutable READY assembly candidate."""
    try:
        source_set = load_ready_inputs(inputs.sources)
        manifest_path = inputs.corpus_root / "assembly-manifest.json"
        with stable_source_descriptor(manifest_path, "assembly-manifest.json"):
            return _validate_loaded(inputs, source_set)
    except ReadyAssemblyError:
        raise
    except ReadyInputError as error:
        raise ReadyAssemblyError(
            ReadyAssemblyFailure.INPUT_INVALID, error.detail
        ) from error
    except (CorpusError, OSError, TypeError, ValueError) as error:
        raise ReadyAssemblyError(
            ReadyAssemblyFailure.VALIDATION_FAILED, str(error)
        ) from error


def _validate_loaded(
    inputs: ReadyValidationInputs,
    source_set: ReadySourceSet,
) -> ReadyAssemblySummary:
    root = inputs.corpus_root
    manifest = read_root_manifest(root)
    validate_copied_inputs(
        root, inputs.sources.plan, inputs.sources.native_inventory_root
    )
    expected_paths, expected_digests = validate_corpora(
        inputs.sources.contract, root, source_set
    )
    validate_support_relations(
        root,
        manifest.get("support_relations"),
        source_set,
        expected_paths,
        expected_digests,
    )
    _validate_bindings(inputs, manifest, expected_digests, source_set.supports)
    records = scan_tree(root)
    if {record.path for record in records} != expected_paths or len(records) != 1484:
        raise ReadyValidationError("assembly path set differs")
    identity = tree_identity(root)
    expected_tree: JsonValue = {
        "files": identity.files,
        "bytes": identity.bytes,
        "sha256": identity.sha256,
    }
    if manifest.get("tree") != expected_tree:
        raise ReadyValidationError("tree identity differs")
    if len(source_set.sources) != 1295 or len(source_set.supports) != 180:
        raise ReadyValidationError("source inventory count differs")
    return ReadyAssemblySummary(
        "VALIDATED",
        7,
        1485,
        1295,
        180,
        identity.files,
        identity.bytes,
        identity.sha256,
        sha256_file(root / "assembly-manifest.json"),
    )


def _validate_bindings(
    inputs: ReadyValidationInputs,
    manifest: dict[str, JsonValue],
    digests: dict[str, str],
    supports: tuple[ReadySupport, ...],
) -> None:
    paths = inputs.sources
    if manifest.get("contract_sha256") != sha256_file(paths.contract):
        raise ReadyValidationError("contract binding differs")
    plan = manifest.get("plan")
    inventory = manifest.get("native_inventory")
    if plan != {
        "path": "conformance-plan.json",
        "sha256": digests["conformance-plan.json"],
    }:
        raise ReadyValidationError("plan binding differs")
    if inventory != {
        "path": "native-unit-inventory.json",
        "sha256": digests["native-unit-inventory.json"],
    }:
        raise ReadyValidationError("inventory binding differs")
    upstream: list[JsonValue] = [
        {"role": role, "sha256": sha256_file(cast(Path, getattr(paths, field)))}
        for role, field in _UPSTREAM_PATHS
    ]
    if manifest.get("upstream_manifests") != upstream:
        raise ReadyValidationError("upstream bindings differ")
    corpus_values: dict[str, JsonValue] = {}
    for document_format in DocumentFormat:
        relative = f"corpora/{document_format.value}/manifest.json"
        support_count = sum(
            getattr(item, "owner_format", None) is document_format for item in supports
        )
        corpus_values[document_format.value] = {
            "path": relative,
            "sha256": digests[relative],
            "conformance_units": 100,
            "blind_files": 75,
            "security_cases": 10,
            "support_files": support_count,
        }
    if manifest.get("corpora") != corpus_values:
        raise ReadyValidationError("corpus bindings differ")
