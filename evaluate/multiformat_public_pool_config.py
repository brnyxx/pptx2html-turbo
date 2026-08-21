from __future__ import annotations

import re
from pathlib import Path

from evaluate.multiformat_corpus_items import object_list, require_keys
from evaluate.multiformat_corpus_sources import validate_identifier
from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_public_pool_types import (
    PublicFormatPlan,
    PublicPoolError,
    PublicSourceGroup,
)
from evaluate.multiformat_schema import (
    JsonValue,
    integer_value,
    object_value,
    string_list,
    string_value,
)
from evaluate.multiformat_strict_json import read_strict_object

COMMIT_PATTERN = re.compile(r"^[0-9a-f]{40}$")
REPOSITORY_PATTERN = re.compile(r"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$")


def load_public_pool_plans(config_path: Path) -> tuple[PublicFormatPlan, ...]:
    values = read_strict_object(config_path)
    require_keys(values, {"schema_version", "formats"}, "public.pool.config")
    if integer_value(values, "schema_version") != 1:
        raise PublicPoolError("public pool config version differs")
    result: list[PublicFormatPlan] = []
    for format_name, format_value in object_value(values, "formats").items():
        if not isinstance(format_value, dict):
            raise PublicPoolError("public pool format config is invalid")
        try:
            document_format = DocumentFormat(format_name)
        except ValueError as error:
            raise PublicPoolError("public pool format is unsupported") from error
        require_keys(
            format_value,
            {"expected_count", "groups"},
            "public.pool.config.format",
        )
        groups = tuple(
            _parse_group(item)
            for item in object_list(
                format_value,
                "groups",
                "public.pool.config.groups",
            )
        )
        expected = integer_value(format_value, "expected_count")
        if (
            len({group.producer for group in groups}) < 5
            or sum(group.quota for group in groups) != expected
        ):
            raise PublicPoolError("public pool group quota differs")
        result.append(PublicFormatPlan(document_format, expected, groups))
    if not result:
        raise PublicPoolError("public pool config has no formats")
    return tuple(sorted(result, key=lambda item: item.document_format.value))


def validate_repository_path(value: str) -> None:
    path = Path(value)
    if (
        not value
        or path.is_absolute()
        or "\\" in value
        or value != path.as_posix()
        or any(part in {"", ".", ".."} for part in path.parts)
    ):
        raise PublicPoolError("public pool repository path is unsafe")


def _parse_group(values: dict[str, JsonValue]) -> PublicSourceGroup:
    require_keys(
        values,
        {
            "producer",
            "repository",
            "commit",
            "license_spdx",
            "quota",
            "path_prefixes",
            "static_paths",
        },
        "public.pool.config.group",
    )
    producer = string_value(values, "producer")
    repository = string_value(values, "repository")
    commit = string_value(values, "commit")
    license_spdx = string_value(values, "license_spdx")
    quota = integer_value(values, "quota")
    validate_identifier(producer, "public.pool.producer")
    if (
        REPOSITORY_PATTERN.fullmatch(repository) is None
        or COMMIT_PATTERN.fullmatch(commit) is None
        or not license_spdx
        or quota <= 0
    ):
        raise PublicPoolError("public pool group identity is invalid")
    prefixes = tuple(string_list(values, "path_prefixes"))
    static_paths = tuple(string_list(values, "static_paths"))
    for path in prefixes:
        validate_repository_path(path.rstrip("/"))
    for path in static_paths:
        validate_repository_path(path)
    return PublicSourceGroup(
        producer,
        repository,
        commit,
        license_spdx,
        quota,
        prefixes,
        static_paths,
    )
