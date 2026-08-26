from dataclasses import dataclass
from enum import StrEnum
from pathlib import Path

from evaluate import multiformat_native_unit_validation as native
from evaluate import multiformat_schema as schema
from evaluate.multiformat_corpus_items import object_list
from evaluate.multiformat_corpus_types import DocumentFormat, SecurityOutcome
from evaluate.multiformat_legacy_binary_validation import validate_legacy_binary_pool
from evaluate.multiformat_public_pool import load_validated_public_pool_sources
from evaluate.multiformat_security_snapshot_validation import validate_security_snapshot
from evaluate.multiformat_strict_json import read_strict_object


class ReadyInputFailure(StrEnum):
    PLAN_INVALID = "plan-invalid"
    CONFORMANCE_INVALID = "conformance-invalid"
    LEGACY_CONFORMANCE_INVALID = "legacy-conformance-invalid"
    LEGACY_BINARY_INVALID = "legacy-binary-invalid"
    PUBLIC_POOL_INVALID = "public-pool-invalid"
    SECURITY_INVALID = "security-invalid"
    NATIVE_INVENTORY_INVALID = "native-inventory-invalid"
    JOIN_INVALID = "join-invalid"
    SOURCE_CHANGED = "source-changed"


@dataclass(frozen=True, slots=True)
class ReadyInputError(Exception):
    failure: ReadyInputFailure
    document_format: DocumentFormat | None
    source_id: str | None
    detail: str


@dataclass(frozen=True, slots=True)
class ReadyInputPaths:
    contract: Path
    plan: Path
    pptx_conformance: Path
    docx_conformance: Path
    xlsx_conformance: Path
    pdf_conformance: Path
    legacy_conformance: Path
    public_config: Path
    public_pool_manifest: Path
    legacy_binary_config: Path
    legacy_binary_manifest: Path
    security_manifest: Path
    routing: Path
    font_manifest: Path
    libreoffice: Path
    pdfinfo: Path
    native_inventory_root: Path


@dataclass(frozen=True, slots=True)
class ReadyBinaryProvenance:
    producer: str
    source_uri: str
    independently_authored: bool


@dataclass(frozen=True, slots=True)
class ReadyConformance:
    upstream_source_id: str
    ordinal: int
    primary_stratum: str
    paired_stratum: str | None
    feature_seed: str
    support_id: str | None
    provenance: ReadyBinaryProvenance | None


@dataclass(frozen=True, slots=True)
class ReadyBlind:
    producer: str
    source_uri: str
    template_family: str
    applicable_metrics: tuple[str, ...]
    background: str


@dataclass(frozen=True, slots=True)
class ReadySecurity:
    case_family: str
    expected_outcome: SecurityOutcome


@dataclass(frozen=True, slots=True)
class ReadySource:
    document_format: DocumentFormat
    source_id: str
    source_path: Path
    source_sha256: str
    unit_count: int
    details: ReadyConformance | ReadyBlind | ReadySecurity


@dataclass(frozen=True, slots=True)
class ReadySupport:
    owner_format: DocumentFormat
    owner_source_id: str
    support_format: DocumentFormat
    modern_case_id: str
    support_id: str
    source_path: Path
    source_sha256: str
    filename: str


@dataclass(frozen=True, slots=True)
class ReadySourceSet:
    sources: tuple[ReadySource, ...]
    supports: tuple[ReadySupport, ...]


def _auxiliary(
    paths: ReadyInputPaths,
    plan: dict[DocumentFormat, list[dict[str, schema.JsonValue]]],
) -> list[ReadySource]:
    validate_legacy_binary_pool(
        paths.legacy_binary_config,
        paths.public_config,
        paths.public_pool_manifest,
        paths.legacy_binary_manifest,
    )
    formats = schema.object_value(
        read_strict_object(paths.legacy_binary_manifest), "formats"
    )
    result: list[ReadySource] = []
    for document_format in (
        DocumentFormat.DOC,
        DocumentFormat.XLS,
        DocumentFormat.PPT,
    ):
        value = schema.object_value(formats, document_format.value)
        for item, case in zip(
            object_list(value, "sources", "ready.binary"),
            plan[document_format][60:],
            strict=True,
        ):
            provenance = ReadyBinaryProvenance(
                schema.string_value(item, "producer"),
                schema.string_value(item, "source_uri"),
                schema.boolean_value(item, "independently_authored"),
            )
            details = ReadyConformance(
                schema.string_value(item, "id"),
                schema.integer_value(case, "ordinal"),
                "binary-specific",
                None,
                schema.sha256_value(case, "feature_seed"),
                None,
                provenance,
            )
            result.append(
                ReadySource(
                    document_format,
                    schema.string_value(case, "id"),
                    paths.legacy_binary_manifest.parent
                    / schema.string_value(item, "path"),
                    schema.sha256_value(item, "sha256"),
                    1,
                    details,
                )
            )
    return result + _blind(paths) + _security(paths)


def _blind(paths: ReadyInputPaths) -> list[ReadySource]:
    public = load_validated_public_pool_sources(
        paths.public_config, paths.public_pool_manifest
    )
    inventory = native.load_native_unit_inventory(
        native.NativeUnitValidationInputs(
            paths.contract,
            paths.public_config,
            paths.public_pool_manifest,
            paths.routing,
            paths.font_manifest,
            paths.libreoffice,
            paths.pdfinfo,
            paths.native_inventory_root,
        )
    )
    counts = {
        (item.document_format, item.source_id): item for item in inventory.sources
    }
    values = schema.object_value(
        read_strict_object(paths.public_pool_manifest), "formats"
    )
    metadata = {
        (document_format, schema.string_value(item, "id")): item
        for document_format in DocumentFormat
        for item in object_list(
            schema.object_value(values, document_format.value),
            "sources",
            "ready.public",
        )
    }
    result: list[ReadySource] = []
    for source in public:
        key = source.document_format, source.source_id
        count, item = counts[key], metadata[key]
        if (count.relative_path, count.source_sha256) != (
            source.relative_path,
            source.source_sha256,
        ):
            raise ReadyInputError(
                ReadyInputFailure.JOIN_INVALID, *key, "blind identity"
            )
        details = ReadyBlind(
            schema.string_value(item, "producer"),
            schema.string_value(item, "source_uri"),
            schema.string_value(item, "template_family"),
            tuple(schema.string_list(item, "applicable_metrics")),
            schema.string_value(item, "background"),
        )
        result.append(
            ReadySource(
                *key,
                paths.public_pool_manifest.parent / source.relative_path,
                source.source_sha256,
                count.unit_count,
                details,
            )
        )
    return result


def _security(paths: ReadyInputPaths) -> list[ReadySource]:
    validate_security_snapshot(paths.contract, paths.security_manifest)
    values = schema.object_value(read_strict_object(paths.security_manifest), "formats")
    result: list[ReadySource] = []
    for document_format in DocumentFormat:
        value = schema.object_value(values, document_format.value)
        for item in object_list(value, "sources", "ready.security"):
            details = ReadySecurity(
                schema.string_value(item, "case_family"),
                SecurityOutcome(schema.string_value(item, "expected_outcome")),
            )
            result.append(
                ReadySource(
                    document_format,
                    schema.string_value(item, "id"),
                    paths.security_manifest.parent / schema.string_value(item, "path"),
                    schema.sha256_value(item, "sha256"),
                    1,
                    details,
                )
            )
    return result


def _finish(sources: list[ReadySource], supports: list[ReadySupport]) -> ReadySourceSet:
    identities = {(item.document_format, item.source_id) for item in sources}
    support_ids = {(item.support_format, item.support_id) for item in supports}
    valid = (
        len(sources) == len(identities) == 1295
        and len(supports) == len(support_ids) == 180
        and not identities & support_ids
    )
    if not valid:
        raise ReadyInputError(ReadyInputFailure.JOIN_INVALID, None, None, "identities")
    order = {ReadyConformance: 0, ReadyBlind: 1, ReadySecurity: 2}
    sources.sort(
        key=lambda item: (
            item.document_format.value,
            order[type(item.details)],
            item.source_id,
        )
    )
    supports.sort(key=lambda item: (item.owner_format.value, item.owner_source_id))
    return ReadySourceSet(tuple(sources), tuple(supports))


def _check(condition: bool, failure: ReadyInputFailure, detail: str) -> None:
    if not condition:
        raise ReadyInputError(failure, None, None, detail)
