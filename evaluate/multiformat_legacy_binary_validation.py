from __future__ import annotations

from collections import Counter
from pathlib import Path

from evaluate.multiformat_corpus_items import (
    canonical_identity,
    canonical_source_uri,
    object_list,
    require_keys,
)
from evaluate.multiformat_corpus_sources import validate_source
from evaluate.multiformat_corpus_types import CorpusError
from evaluate.multiformat_legacy_binary_config import (
    LegacyBinaryPoolError,
    load_legacy_binary_plans,
)
from evaluate.multiformat_public_pool import validate_public_pool
from evaluate.multiformat_public_pool_sources import public_source_url
from evaluate.multiformat_public_pool_types import PublicPoolError
from evaluate.multiformat_schema import (
    JsonValue,
    boolean_value,
    integer_value,
    object_value,
    sha256_file,
    sha256_value,
    string_list,
    string_value,
)
from evaluate.multiformat_strict_json import StrictJsonError, read_strict_object

_SOURCE_FIELDS = {
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
    "independently_authored",
}


def validate_legacy_binary_pool(
    config_path: Path,
    public_config_path: Path,
    blind_manifest_path: Path,
    manifest_path: Path,
) -> dict[str, int]:
    try:
        return _validate(
            config_path,
            public_config_path,
            blind_manifest_path,
            manifest_path,
        )
    except LegacyBinaryPoolError:
        raise
    except (
        CorpusError,
        OSError,
        PublicPoolError,
        StrictJsonError,
        ValueError,
    ) as error:
        raise LegacyBinaryPoolError("legacy binary pool validation failed") from error


def _validate(
    config_path: Path,
    public_config_path: Path,
    blind_manifest_path: Path,
    manifest_path: Path,
) -> dict[str, int]:
    validate_public_pool(public_config_path, blind_manifest_path)
    plans = load_legacy_binary_plans(config_path, public_config_path)
    blind = read_strict_object(blind_manifest_path)
    values = read_strict_object(manifest_path)
    require_keys(
        values,
        {
            "schema_version",
            "status",
            "selection_config_sha256",
            "source_catalog_sha256",
            "blind_manifest_sha256",
            "formats",
        },
        "legacy.binary.pool",
    )
    if (
        integer_value(values, "schema_version") != 1
        or string_value(values, "status") != "COLLECTED"
        or sha256_value(values, "selection_config_sha256") != sha256_file(config_path)
        or sha256_value(values, "source_catalog_sha256")
        != sha256_file(public_config_path)
        or sha256_value(values, "blind_manifest_sha256")
        != sha256_file(blind_manifest_path)
    ):
        raise LegacyBinaryPoolError("legacy binary pool binding differs")
    format_values = object_value(values, "formats")
    if set(format_values) != {plan.document_format.value for plan in plans}:
        raise LegacyBinaryPoolError("legacy binary pool format set differs")
    blind_hashes, blind_uris, blind_origins = _blind_identities(blind)
    seen_hashes = set(blind_hashes)
    seen_uris = set(blind_uris)
    seen_origins = set(blind_origins)
    expected_paths = {manifest_path.resolve(strict=True)}
    counts: dict[str, int] = {}
    for plan in plans:
        name = plan.document_format.value
        format_value = object_value(format_values, name)
        require_keys(
            format_value,
            {"expected_count", "sources"},
            "legacy.binary.pool.format",
        )
        sources = object_list(
            format_value,
            "sources",
            "legacy.binary.pool.sources",
        )
        if (
            integer_value(format_value, "expected_count") != plan.expected_count
            or len(sources) != plan.expected_count
        ):
            raise LegacyBinaryPoolError("legacy binary source count differs")
        quotas = Counter({group.producer: group.quota for group in plan.groups})
        observed: Counter[str] = Counter()
        groups = {group.producer: group for group in plan.groups}
        for source_value in sources:
            require_keys(
                source_value,
                _SOURCE_FIELDS,
                "legacy.binary.pool.source",
            )
            producer = string_value(source_value, "producer")
            group = groups.get(producer)
            if group is None:
                raise LegacyBinaryPoolError("legacy binary producer differs")
            source = validate_source(
                source_value,
                manifest_path.parent,
                plan.document_format,
                require_valid_format=True,
            )
            uri = string_value(source_value, "source_uri")
            origin = (
                string_value(source_value, "repository"),
                string_value(source_value, "commit"),
                string_value(source_value, "repository_path"),
            )
            if (
                source.item_id
                != f"binary-{name}-{producer}-{observed[producer] + 1:03d}"
                or not boolean_value(source_value, "independently_authored")
                or string_list(source_value, "applicable_metrics")
                != ["visual", "content", "layout"]
                or string_value(source_value, "background") != "light"
                or origin[:2] != (group.repository, group.commit)
                or string_value(source_value, "license_spdx") != group.license_spdx
                or uri != public_source_url(*origin)
            ):
                raise LegacyBinaryPoolError("legacy binary source provenance differs")
            canonical_identity(producer, "legacy.binary.producer")
            canonical_identity(
                string_value(source_value, "template_family"),
                "legacy.binary.template",
            )
            canonical_source_uri(uri)
            if (
                source.digest in seen_hashes
                or uri in seen_uris
                or origin in seen_origins
            ):
                raise LegacyBinaryPoolError("legacy binary source overlap")
            seen_hashes.add(source.digest)
            seen_uris.add(uri)
            seen_origins.add(origin)
            observed[producer] += 1
            expected_paths.add(
                (manifest_path.parent / source.relative_path).resolve(strict=True)
            )
        if observed != quotas:
            raise LegacyBinaryPoolError("legacy binary group quota differs")
        counts[name] = len(sources)
    actual_paths = {
        path.resolve(strict=True)
        for path in manifest_path.parent.rglob("*")
        if path.is_file() or path.is_symlink()
    }
    if actual_paths != expected_paths:
        raise LegacyBinaryPoolError("legacy binary pool file set differs")
    return counts


def _blind_identities(
    blind: dict[str, JsonValue],
) -> tuple[set[str], set[str], set[tuple[str, str, str]]]:
    hashes: set[str] = set()
    uris: set[str] = set()
    origins: set[tuple[str, str, str]] = set()
    for value in object_value(blind, "formats").values():
        if not isinstance(value, dict):
            raise LegacyBinaryPoolError("blind pool format is invalid")
        for source in object_list(value, "sources", "blind.pool.sources"):
            hashes.add(sha256_value(source, "sha256"))
            uris.add(string_value(source, "source_uri"))
            origins.add(
                (
                    string_value(source, "repository"),
                    string_value(source, "commit"),
                    string_value(source, "repository_path"),
                )
            )
    return hashes, uris, origins
