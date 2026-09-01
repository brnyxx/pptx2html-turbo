from __future__ import annotations

from collections import defaultdict
from pathlib import Path

from evaluate.multiformat_ready_assembly_types import ReadyValidationError
from evaluate.multiformat_ready_types import ReadyConformance, ReadySourceSet
from evaluate.multiformat_schema import JsonValue, sha256_file

RELATION_FIELDS = {
    "owner_format",
    "owner_source_id",
    "support_format",
    "modern_case_id",
    "support_id",
    "path",
    "sha256",
}


def validate_support_relations(
    root: Path,
    values: JsonValue | None,
    sources: ReadySourceSet,
    expected_paths: set[str],
    expected_digests: dict[str, str],
) -> None:
    if not isinstance(values, list) or len(values) != 180:
        raise ReadyValidationError("support relation count")
    expected: list[dict[str, JsonValue]] = []
    modern = {
        (item.document_format, item.source_id): item
        for item in sources.sources
        if isinstance(item.details, ReadyConformance)
    }
    for support in sorted(
        sources.supports,
        key=lambda item: (
            item.owner_format.value,
            item.owner_source_id,
            item.support_id,
        ),
    ):
        derived_id = f"{support.owner_format.value}-support-{support.modern_case_id}"
        path = (
            f"corpora/{support.owner_format.value}/sources/support/{support.filename}"
        )
        selected = modern.get((support.support_format, support.modern_case_id))
        if (
            support.support_id != derived_id
            or support.filename != f"{derived_id}.{support.support_format.value}"
            or selected is None
            or selected.source_sha256 != support.source_sha256
        ):
            raise ReadyValidationError("support source binding")
        expected.append(
            {
                "owner_format": support.owner_format.value,
                "owner_source_id": support.owner_source_id,
                "support_format": support.support_format.value,
                "modern_case_id": support.modern_case_id,
                "support_id": support.support_id,
                "path": path,
                "sha256": support.source_sha256,
            }
        )
        if sha256_file(root / path) != support.source_sha256:
            raise ReadyValidationError(f"support digest differs: {support.support_id}")
        expected_paths.add(path)
        expected_digests[path] = support.source_sha256
    if values != expected:
        raise ReadyValidationError("support relations differ")
    _validate_digest_reuse(sources)


def _validate_digest_reuse(sources: ReadySourceSet) -> None:
    occurrences: dict[str, list[tuple[str, str]]] = defaultdict(list)
    for source in sources.sources:
        occurrences[source.source_sha256].append(
            (source.document_format.value, source.source_id)
        )
    for support in sources.supports:
        occurrences[support.source_sha256].append(
            (support.support_format.value, support.support_id)
        )
    relation_digests = {item.source_sha256 for item in sources.supports}
    for digest, identities in occurrences.items():
        expected = 2 if digest in relation_digests else 1
        if len(identities) != expected:
            raise ReadyValidationError("unapproved cross-manifest digest reuse")
        if len(set(identities)) != len(identities):
            raise ReadyValidationError("support identity reuses a primary identity")
