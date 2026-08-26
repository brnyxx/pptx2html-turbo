import tempfile
from pathlib import Path

from evaluate import build_multiformat_conformance_plan as conformance_plan
from evaluate import multiformat_schema as schema
from evaluate.multiformat_corpus_items import object_list
from evaluate.multiformat_corpus_sources import validate_source
from evaluate.multiformat_corpus_types import CorpusError, DocumentFormat
from evaluate.multiformat_ready_tree import tree_identity
from evaluate.multiformat_ready_identity import (
    file_identity,
    identities_unchanged,
    input_roots,
    source_file_identities,
)
from evaluate.multiformat_ready_types import (
    ReadyConformance,
    ReadyInputError,
    ReadyInputFailure,
    ReadyInputPaths,
    ReadySource,
    ReadySourceSet,
    ReadySupport,
    _auxiliary,
    _check,
    _finish,
)
from evaluate.multiformat_strict_json import read_strict_object

Plan = dict[DocumentFormat, list[dict[str, schema.JsonValue]]]
_PAIRS = {
    DocumentFormat.DOC: DocumentFormat.DOCX,
    DocumentFormat.XLS: DocumentFormat.XLSX,
    DocumentFormat.PPT: DocumentFormat.PPTX,
}
_MODERN = (*_PAIRS.values(), DocumentFormat.PDF)


def _before_final_snapshot_identity(label: str, root: Path | None) -> None:
    del label, root


def load_ready_inputs(paths: ReadyInputPaths) -> ReadySourceSet:
    try:
        roots = input_roots(paths)
        initial = {name: tree_identity(root) for name, root in roots.items()}
        plan = _plan(paths)
        modern = _modern(paths, plan)
        paired, supports = _paired(paths, plan, modern)
        sources = modern + paired + _auxiliary(paths, plan)
        source_identities = source_file_identities(sources, supports)
        watched = {
            path: file_identity(path)
            for root in roots.values()
            for path in root.glob("*.json")
        }
        for name, root in roots.items():
            _before_final_snapshot_identity(name, root)
            _check(
                tree_identity(root) == initial[name],
                ReadyInputFailure.SOURCE_CHANGED,
                name,
            )
        _check(
            identities_unchanged(watched),
            ReadyInputFailure.SOURCE_CHANGED,
            "file",
        )
        _check(
            identities_unchanged(source_identities),
            ReadyInputFailure.SOURCE_CHANGED,
            "source",
        )
        return _finish(sources, supports)
    except ReadyInputError:
        raise
    except (
        conformance_plan.ConformancePlanError,
        CorpusError,
        OSError,
        TypeError,
        ValueError,
    ) as error:
        raise ReadyInputError(
            ReadyInputFailure.JOIN_INVALID, None, None, "upstream validation"
        ) from error


def _plan(paths: ReadyInputPaths) -> Plan:
    with tempfile.TemporaryDirectory() as directory:
        generated = Path(directory) / "plan.json"
        conformance_plan.build_conformance_plan(paths.contract, generated)
        _check(
            generated.read_bytes() == paths.plan.read_bytes(),
            ReadyInputFailure.PLAN_INVALID,
            "bytes",
        )
    conformance_plan.validate_conformance_plan(paths.contract, paths.plan)
    values = schema.object_value(read_strict_object(paths.plan), "formats")
    result = {
        DocumentFormat(name): object_list(value, "cases", "ready.plan")
        for name, value in values.items()
        if isinstance(value, dict)
    }
    _check(
        schema.sha256_file(paths.plan)
        == "609762e81c90f4d2185f7078fad699aa1ea65c76b2aa2c48680b7b001e6df94a",
        ReadyInputFailure.PLAN_INVALID,
        "identity",
    )
    return result


def _modern(paths: ReadyInputPaths, plan: Plan) -> list[ReadySource]:
    manifests = (
        paths.docx_conformance,
        paths.xlsx_conformance,
        paths.pptx_conformance,
        paths.pdf_conformance,
    )
    result: list[ReadySource] = []
    for document_format, manifest in zip(_MODERN, manifests, strict=True):
        values = read_strict_object(manifest)
        status = "FROZEN" if document_format is DocumentFormat.DOCX else "GENERATED"
        bound = (
            schema.string_value(values, "status"),
            schema.string_value(values, "format"),
            schema.sha256_value(values, "contract_sha256"),
            schema.sha256_value(values, "plan_sha256"),
        )
        _check(
            bound
            == (
                status,
                document_format.value,
                schema.sha256_file(paths.contract),
                schema.sha256_file(paths.plan),
            ),
            ReadyInputFailure.CONFORMANCE_INVALID,
            "root",
        )
        items = object_list(values, "files", "ready.modern")
        _check(len(items) == 100, ReadyInputFailure.CONFORMANCE_INVALID, "count")
        for item, case in zip(items, plan[document_format], strict=True):
            source_id = schema.string_value(case, "id")
            joined = (
                schema.string_value(item, "id") == source_id
                and schema.string_value(item, "primary_stratum")
                == schema.string_value(case, "primary_stratum")
                and schema.integer_value(item, "unit_count") == 1
            )
            _check(joined, ReadyInputFailure.CONFORMANCE_INVALID, source_id)
            source = _source(manifest, item, document_format)
            details = ReadyConformance(
                source_id,
                schema.integer_value(case, "ordinal"),
                schema.string_value(case, "primary_stratum"),
                None,
                schema.sha256_value(case, "feature_seed"),
                None,
                None,
            )
            result.append(
                ReadySource(
                    document_format,
                    source_id,
                    source,
                    schema.sha256_value(item, "sha256"),
                    1,
                    details,
                )
            )
    return result


def _paired(
    paths: ReadyInputPaths, plan: Plan, modern: list[ReadySource]
) -> tuple[list[ReadySource], list[ReadySupport]]:
    manifest = paths.legacy_conformance
    values = read_strict_object(manifest)
    bound = (
        schema.string_value(values, "status") == "GENERATED"
        and schema.sha256_value(values, "contract_sha256")
        == schema.sha256_file(paths.contract)
        and schema.sha256_value(values, "plan_sha256") == schema.sha256_file(paths.plan)
    )
    _check(bound, ReadyInputFailure.LEGACY_CONFORMANCE_INVALID, "root")
    formats = schema.object_value(values, "formats")
    modern_map = {(item.document_format, item.source_id): item for item in modern}
    result: list[ReadySource] = []
    supports: list[ReadySupport] = []
    for owner, support_format in _PAIRS.items():
        value = schema.object_value(formats, owner.value)
        items = object_list(value, "files", "ready.legacy")
        _check(
            len(items) == 60
            and schema.string_value(value, "paired_format") == support_format.value,
            ReadyInputFailure.LEGACY_CONFORMANCE_INVALID,
            "count",
        )
        for item, case in zip(items, plan[owner][:60], strict=True):
            source_id, modern_id = (
                schema.string_value(case, "id"),
                schema.string_value(case, "paired_case_id"),
            )
            paired = schema.object_value(item, "paired_source")
            selected = modern_map[support_format, modern_id]
            joined = (
                schema.string_value(item, "id") == source_id
                and schema.string_value(paired, "id") == modern_id
                and schema.sha256_value(paired, "sha256") == selected.source_sha256
            )
            _check(joined, ReadyInputFailure.LEGACY_CONFORMANCE_INVALID, source_id)
            source, support = (
                _source(manifest, item, owner),
                _source(manifest, paired, support_format),
            )
            support_id = f"{owner.value}-support-{modern_id}"
            details = ReadyConformance(
                source_id,
                schema.integer_value(case, "ordinal"),
                "paired-legacy",
                schema.string_value(case, "paired_stratum"),
                schema.sha256_value(case, "feature_seed"),
                support_id,
                None,
            )
            result.append(
                ReadySource(
                    owner,
                    source_id,
                    source,
                    schema.sha256_value(item, "sha256"),
                    1,
                    details,
                )
            )
            supports.append(
                ReadySupport(
                    owner,
                    source_id,
                    support_format,
                    modern_id,
                    support_id,
                    support,
                    selected.source_sha256,
                    f"{support_id}.{support_format.value}",
                )
            )
    return result, supports


def _source(
    manifest: Path, item: dict[str, schema.JsonValue], document_format: DocumentFormat
) -> Path:
    validate_source(item, manifest.parent, document_format, require_valid_format=True)
    return manifest.parent / schema.string_value(item, "path")
