from __future__ import annotations

import shutil
from collections import Counter
from pathlib import Path

from evaluate.multiformat_candidate_artifacts import write_canonical_json
from evaluate.multiformat_corpus_items import object_list, require_keys
from evaluate.multiformat_corpus_sources import validate_source
from evaluate.multiformat_corpus_types import (
    CorpusError,
    DocumentFormat,
    SourceRecord,
)
from evaluate.multiformat_public_pool_config import load_public_pool_plans
from evaluate.multiformat_public_pool_sources import collect_public_source_group
from evaluate.multiformat_public_pool_types import (
    BlobFetcher,
    PublicFormatPlan,
    PublicPoolError,
    PublicSourceGroup,
    TreeFetcher,
    ValidatedPublicPoolSource,
)
from evaluate.multiformat_schema import (
    JsonValue,
    integer_value,
    object_value,
    string_value,
)
from evaluate.multiformat_strict_json import StrictJsonError, read_strict_object


def collect_public_pool(
    config_path: Path,
    output_dir: Path,
    *,
    tree_fetcher: TreeFetcher,
    blob_fetcher: BlobFetcher,
) -> Path:
    plans = load_public_pool_plans(config_path)
    if output_dir.exists():
        raise PublicPoolError("public pool output already exists")
    output_dir.mkdir(parents=True)
    try:
        seen_hashes: set[str] = set()
        trees: dict[tuple[str, str], dict[str, JsonValue]] = {}
        formats: dict[str, JsonValue] = {}
        for plan in plans:
            sources: list[JsonValue] = []
            for group in plan.groups:
                tree_key = (group.repository, group.commit)
                if tree_key not in trees:
                    trees[tree_key] = tree_fetcher(*tree_key)
                sources.extend(
                    collect_public_source_group(
                        output_dir,
                        plan.document_format,
                        group,
                        trees[tree_key],
                        blob_fetcher,
                        seen_hashes,
                    )
                )
            if len(sources) != plan.expected_count:
                raise PublicPoolError("public pool format count differs")
            format_value: dict[str, JsonValue] = {
                "expected_count": plan.expected_count,
                "sources": sources,
            }
            formats[plan.document_format.value] = format_value
        manifest = output_dir / "public-pool.json"
        write_canonical_json(
            manifest,
            {
                "schema_version": 1,
                "status": "COLLECTED",
                "formats": formats,
            },
        )
        validate_public_pool(config_path, manifest)
        return manifest
    except PublicPoolError:
        shutil.rmtree(output_dir)
        raise
    except (CorpusError, OSError, TypeError, ValueError) as error:
        shutil.rmtree(output_dir)
        raise PublicPoolError("public pool collection failed") from error


def load_validated_public_pool_sources(
    config_path: Path,
    manifest_path: Path,
) -> tuple[ValidatedPublicPoolSource, ...]:
    try:
        return _load_validated_public_pool_sources(config_path, manifest_path)
    except PublicPoolError:
        raise
    except (
        CorpusError,
        OSError,
        StrictJsonError,
        TypeError,
        ValueError,
    ) as error:
        raise PublicPoolError("public pool validation failed") from error


def validate_public_pool(config_path: Path, manifest_path: Path) -> None:
    _ = load_validated_public_pool_sources(config_path, manifest_path)


def _load_validated_public_pool_sources(
    config_path: Path,
    manifest_path: Path,
) -> tuple[ValidatedPublicPoolSource, ...]:
    plans = load_public_pool_plans(config_path)
    root = manifest_path.resolve(strict=True).parent
    values = read_strict_object(manifest_path)
    require_keys(values, {"schema_version", "status", "formats"}, "public.pool")
    if (
        integer_value(values, "schema_version") != 1
        or string_value(values, "status") != "COLLECTED"
    ):
        raise PublicPoolError("public pool status is invalid")
    formats = object_value(values, "formats")
    if set(formats) != {plan.document_format.value for plan in plans}:
        raise PublicPoolError("public pool format set differs")
    expected_files = {manifest_path.resolve(strict=True)}
    seen_keys: set[tuple[DocumentFormat, str]] = set()
    seen_paths: set[str] = set()
    seen_hashes: set[str] = set()
    result: list[ValidatedPublicPoolSource] = []
    for plan in plans:
        source_values = object_value(formats, plan.document_format.value)
        require_keys(
            source_values,
            {"expected_count", "sources"},
            "public.pool.format",
        )
        sources = object_list(source_values, "sources", "public.pool.sources")
        if (
            integer_value(source_values, "expected_count") != plan.expected_count
            or len(sources) != plan.expected_count
        ):
            raise PublicPoolError("public pool source count differs")
        groups = {group.producer: group for group in plan.groups}
        group_counts: Counter[str] = Counter()
        for item in sources:
            source = _validate_pool_source(item, root, plan, groups)
            key = (plan.document_format, source.item_id)
            if key in seen_keys:
                raise PublicPoolError("public pool source key is duplicated")
            if source.relative_path in seen_paths:
                raise PublicPoolError("public pool source path is duplicated")
            if source.digest in seen_hashes:
                raise PublicPoolError("public pool source bytes are duplicated")
            seen_keys.add(key)
            seen_paths.add(source.relative_path)
            seen_hashes.add(source.digest)
            group_counts[string_value(item, "producer")] += 1
            expected_files.add((root / source.relative_path).resolve(strict=True))
            result.append(
                ValidatedPublicPoolSource(
                    plan.document_format,
                    source.item_id,
                    source.relative_path,
                    source.digest,
                )
            )
        if group_counts != Counter(
            {group.producer: group.quota for group in plan.groups}
        ):
            raise PublicPoolError("public pool producer quota differs")
    actual_files = {
        path for path in root.rglob("*") if path.is_file() or path.is_symlink()
    }
    if actual_files != expected_files:
        raise PublicPoolError("public pool file set is not exact")
    return tuple(
        sorted(
            result,
            key=lambda item: (item.document_format.value, item.source_id),
        )
    )


def _validate_pool_source(
    values: dict[str, JsonValue],
    root: Path,
    plan: PublicFormatPlan,
    groups: dict[str, PublicSourceGroup],
) -> SourceRecord:
    require_keys(
        values,
        {
            "id",
            "path",
            "sha256",
            "producer",
            "source_uri",
            "template_family",
            "repository",
            "commit",
            "repository_path",
            "license_spdx",
            "applicable_metrics",
            "background",
        },
        "public.pool.source",
    )
    producer = string_value(values, "producer")
    group = groups.get(producer)
    if group is None or (
        string_value(values, "repository") != group.repository
        or string_value(values, "commit") != group.commit
        or string_value(values, "license_spdx") != group.license_spdx
    ):
        raise PublicPoolError("public pool source provenance differs")
    return validate_source(
        values,
        root,
        plan.document_format,
        require_valid_format=True,
    )


__all__ = [
    "PublicPoolError",
    "collect_public_pool",
    "load_validated_public_pool_sources",
    "validate_public_pool",
]
