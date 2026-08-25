from __future__ import annotations

import hashlib
import stat
from dataclasses import dataclass
from pathlib import Path

from evaluate.jcs import canonicalize
from evaluate.multiformat_corpus_items import object_list, require_keys
from evaluate.multiformat_corpus_types import CorpusError, DocumentFormat
from evaluate.multiformat_font_snapshot import (
    FontSnapshotError,
    FontSnapshotSummary,
    validate_font_snapshot,
)
from evaluate.multiformat_native_unit_execution_validation import (
    NativeExecutionBindings,
)
from evaluate.multiformat_native_unit_record_validation import (
    source_failure,
    validate_scoped_source,
)
from evaluate.multiformat_native_unit_stable_validation import stable_bytes
from evaluate.multiformat_native_unit_tool_validation import (
    validate_pdf_count as _validate_pdf_count,
)
from evaluate.multiformat_native_unit_tool_validation import (
    validate_platform,
    validate_runtime_bindings,
)
from evaluate.multiformat_native_unit_types import NativeUnitError, NativeUnitFailure
from evaluate.multiformat_public_pool import load_validated_public_pool_sources
from evaluate.multiformat_public_pool_types import PublicPoolError
from evaluate.multiformat_reference_routing import RoutingError, load_reference_routing
from evaluate.multiformat_schema import (
    JsonValue,
    integer_value,
    object_value,
    sha256_file,
    sha256_value,
    string_value,
)
from evaluate.multiformat_strict_json import MAX_JSON_BYTES, parse_strict_object_bytes


@dataclass(frozen=True, slots=True)
class NativeUnitValidationInputs:
    contract: Path
    public_config: Path
    public_pool_manifest: Path
    routing: Path
    font_manifest: Path
    libreoffice: Path
    pdfinfo: Path
    inventory_root: Path


@dataclass(frozen=True, slots=True)
class NativeUnitInventorySummary:
    files: int
    sources: int
    observations: int
    total_units: int
    manifest_sha256: str


@dataclass(frozen=True, slots=True)
class NativeUnitCount:
    document_format: DocumentFormat
    source_id: str
    relative_path: str
    source_sha256: str
    unit_count: int


@dataclass(frozen=True, slots=True)
class NativeUnitInventory:
    summary: NativeUnitInventorySummary
    sources: tuple[NativeUnitCount, ...]


def validate_native_unit_inventory(
    inputs: NativeUnitValidationInputs,
) -> NativeUnitInventorySummary:
    return load_native_unit_inventory(inputs).summary


def load_native_unit_inventory(
    inputs: NativeUnitValidationInputs,
) -> NativeUnitInventory:
    try:
        operating_system, architecture = validate_platform()
        expected_sources = load_validated_public_pool_sources(
            inputs.public_config,
            inputs.public_pool_manifest,
        )
        routing = load_reference_routing(inputs.routing)
        font = validate_font_snapshot(inputs.font_manifest, inputs.font_manifest.parent)
        root_information = inputs.inventory_root.lstat()
        if not stat.S_ISDIR(root_information.st_mode):
            raise _failure("inventory root is not a directory")
        root = inputs.inventory_root.resolve(strict=True)
        manifest = root / "native-unit-inventory.json"
        _manifest_file, manifest_bytes = stable_bytes(
            manifest,
            executable=False,
            maximum=MAX_JSON_BYTES,
        )
        values = parse_strict_object_bytes(manifest_bytes)
        if manifest_bytes != canonicalize(values) + b"\n":
            raise _failure("inventory manifest is not canonical JCS")
        require_keys(
            values,
            {
                "schema_version",
                "status",
                "contract_sha256",
                "public_pool",
                "routing",
                "tools",
                "font",
                "runtime",
                "sources",
            },
            "native.inventory",
        )
        _validate_root(inputs, values, routing.sha256, font)
        pdfinfo_tool = validate_runtime_bindings(
            values,
            inputs.libreoffice,
            inputs.pdfinfo,
            operating_system,
            architecture,
        )
        execution_bindings = NativeExecutionBindings(
            routing,
            object_value(values, "tools"),
            sha256_value(object_value(values, "font"), "environment_sha256"),
        )
        source_values = object_list(values, "sources", "native.inventory.sources")
        if len(source_values) != 525 or len(expected_sources) != 525:
            raise _failure("inventory source count differs")
        expected_files = {"native-unit-inventory.json"}
        counts: list[NativeUnitCount] = []
        nonces: set[str] = set()
        for expected, source in zip(expected_sources, source_values, strict=True):
            count, bindings = validate_scoped_source(
                root,
                inputs.pdfinfo,
                pdfinfo_tool,
                expected,
                source,
                nonces,
                execution_bindings,
                _validate_pdf_count,
            )
            if expected_files & bindings:
                raise source_failure(expected, "inventory evidence path is duplicated")
            expected_files.update(bindings)
            counts.append(
                NativeUnitCount(
                    expected.document_format,
                    expected.source_id,
                    expected.relative_path,
                    expected.source_sha256,
                    count,
                )
            )
        _validate_tree(root, expected_files)
        summary = NativeUnitInventorySummary(
            files=len(expected_files),
            sources=len(counts),
            observations=2 * len(counts),
            total_units=sum(item.unit_count for item in counts),
            manifest_sha256=hashlib.sha256(manifest_bytes).hexdigest(),
        )
        return NativeUnitInventory(summary, tuple(counts))
    except NativeUnitError:
        raise
    except (
        CorpusError,
        FontSnapshotError,
        PublicPoolError,
        RoutingError,
        OSError,
        TypeError,
        ValueError,
    ) as error:
        raise _failure("native inventory validation failed") from error


def _validate_root(
    inputs: NativeUnitValidationInputs,
    values: dict[str, JsonValue],
    routing_sha256: str,
    font: FontSnapshotSummary,
) -> None:
    if (
        integer_value(values, "schema_version") != 1
        or string_value(values, "status") != "CAPTURED"
        or sha256_value(values, "contract_sha256") != sha256_file(inputs.contract)
    ):
        raise _failure("inventory root identity differs")
    public = object_value(values, "public_pool")
    require_keys(public, {"config_sha256", "manifest_sha256"}, "public_pool")
    if sha256_value(public, "config_sha256") != sha256_file(
        inputs.public_config
    ) or sha256_value(public, "manifest_sha256") != sha256_file(
        inputs.public_pool_manifest
    ):
        raise _failure("public pool binding differs")
    route = object_value(values, "routing")
    require_keys(route, {"sha256"}, "routing")
    if sha256_value(route, "sha256") != routing_sha256:
        raise _failure("routing binding differs")
    font_value = object_value(values, "font")
    require_keys(font_value, {"manifest_sha256", "environment_sha256"}, "font")
    if (
        sha256_value(font_value, "manifest_sha256") != font.manifest_sha256
        or sha256_value(font_value, "environment_sha256") != font.environment_sha256
    ):
        raise _failure("font binding differs")


def _validate_tree(root: Path, expected: set[str]) -> None:
    actual: set[str] = set()
    identities: set[tuple[int, int]] = set()
    for path in root.rglob("*"):
        value = path.lstat()
        if stat.S_ISDIR(value.st_mode):
            continue
        if not stat.S_ISREG(value.st_mode) or value.st_nlink != 1:
            raise _failure("inventory tree contains an invalid file")
        identity = (value.st_dev, value.st_ino)
        if identity in identities:
            raise _failure("inventory tree reuses a file inode")
        identities.add(identity)
        actual.add(path.relative_to(root).as_posix())
    if actual != expected:
        raise _failure("inventory file set differs")


def _failure(detail: str) -> NativeUnitError:
    return NativeUnitError(NativeUnitFailure.OUTPUT_INVALID, None, None, detail)
