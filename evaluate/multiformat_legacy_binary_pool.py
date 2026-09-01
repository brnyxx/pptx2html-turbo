from __future__ import annotations

import tempfile
from pathlib import Path

from evaluate.multiformat_candidate_artifacts import write_canonical_json
from evaluate.multiformat_corpus_items import object_list
from evaluate.multiformat_legacy_binary_config import (
    LegacyBinaryPoolError,
    load_legacy_binary_plans,
)
from evaluate.multiformat_legacy_binary_validation import (
    validate_legacy_binary_pool,
)
from evaluate.multiformat_public_pool import validate_public_pool
from evaluate.multiformat_public_pool_sources import collect_public_source_group
from evaluate.multiformat_public_pool_types import (
    BlobFetcher,
    PublicPoolError,
    TreeFetcher,
)
from evaluate.multiformat_schema import (
    JsonValue,
    object_value,
    sha256_file,
    sha256_value,
    string_value,
)
from evaluate.multiformat_strict_json import (
    StrictJsonError,
    read_strict_object,
)


def collect_legacy_binary_pool(
    config_path: Path,
    public_config_path: Path,
    blind_manifest_path: Path,
    output_dir: Path,
    *,
    tree_fetcher: TreeFetcher,
    blob_fetcher: BlobFetcher,
) -> Path:
    if output_dir.exists():
        raise LegacyBinaryPoolError("legacy binary pool output already exists")
    try:
        validate_public_pool(public_config_path, blind_manifest_path)
        blind_sha256 = sha256_file(blind_manifest_path)
        blind = read_strict_object(blind_manifest_path)
        seen_hashes, excluded = _blind_identities(blind)
        plans = load_legacy_binary_plans(config_path, public_config_path)
        output_dir.parent.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(
            prefix=".legacy-binary-pool-",
            dir=output_dir.parent,
        ) as temp_dir:
            staging = Path(temp_dir) / "pool"
            staging.mkdir()
            trees: dict[tuple[str, str], dict[str, JsonValue]] = {}
            formats: dict[str, JsonValue] = {}
            for plan in plans:
                sources: list[dict[str, JsonValue]] = []
                for group in plan.groups:
                    key = (group.repository, group.commit)
                    if key not in trees:
                        trees[key] = tree_fetcher(*key)

                    def filtered_blob(
                        repository: str,
                        commit: str,
                        path: str,
                    ) -> bytes:
                        if (repository, commit, path) in excluded:
                            return b""
                        return blob_fetcher(repository, commit, path)

                    try:
                        group_sources = collect_public_source_group(
                            staging,
                            plan.document_format,
                            group,
                            trees[key],
                            filtered_blob,
                            seen_hashes,
                        )
                    except PublicPoolError as error:
                        if "quota" in str(error):
                            raise LegacyBinaryPoolError(
                                "legacy binary source shortage: "
                                f"{plan.document_format.value}/{group.producer}"
                            ) from error
                        raise
                    for index, source in enumerate(group_sources, start=1):
                        source["id"] = (
                            f"binary-{plan.document_format.value}-"
                            f"{group.producer}-{index:03d}"
                        )
                        source["independently_authored"] = True
                    sources.extend(group_sources)
                if len(sources) != plan.expected_count:
                    raise LegacyBinaryPoolError("legacy binary source count differs")
                source_values: list[JsonValue] = list(sources)
                format_value: dict[str, JsonValue] = {
                    "expected_count": plan.expected_count,
                    "sources": source_values,
                }
                formats[plan.document_format.value] = format_value
            manifest = staging / "legacy-binary-pool.json"
            write_canonical_json(
                manifest,
                {
                    "schema_version": 1,
                    "status": "COLLECTED",
                    "selection_config_sha256": sha256_file(config_path),
                    "source_catalog_sha256": sha256_file(public_config_path),
                    "blind_manifest_sha256": blind_sha256,
                    "formats": formats,
                },
            )
            validate_legacy_binary_pool(
                config_path,
                public_config_path,
                blind_manifest_path,
                manifest,
            )
            validate_public_pool(public_config_path, blind_manifest_path)
            if sha256_file(blind_manifest_path) != blind_sha256:
                raise LegacyBinaryPoolError("blind pool changed during collection")
            staging.rename(output_dir)
        return output_dir / "legacy-binary-pool.json"
    except LegacyBinaryPoolError:
        raise
    except (
        OSError,
        PublicPoolError,
        StrictJsonError,
        ValueError,
    ) as error:
        raise LegacyBinaryPoolError("legacy binary collection failed") from error


def _blind_identities(
    blind: dict[str, JsonValue],
) -> tuple[set[str], set[tuple[str, str, str]]]:
    hashes: set[str] = set()
    origins: set[tuple[str, str, str]] = set()
    for value in object_value(blind, "formats").values():
        if not isinstance(value, dict):
            raise LegacyBinaryPoolError("blind pool format is invalid")
        for source in object_list(value, "sources", "blind.pool.sources"):
            hashes.add(sha256_value(source, "sha256"))
            origins.add(
                (
                    string_value(source, "repository"),
                    string_value(source, "commit"),
                    string_value(source, "repository_path"),
                )
            )
    return hashes, origins


__all__ = [
    "LegacyBinaryPoolError",
    "collect_legacy_binary_pool",
    "load_legacy_binary_plans",
    "validate_legacy_binary_pool",
]
