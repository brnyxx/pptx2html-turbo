from __future__ import annotations

from dataclasses import replace
from pathlib import Path

from evaluate.multiformat_corpus_items import object_list, require_keys
from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_public_pool_config import load_public_pool_plans
from evaluate.multiformat_public_pool_types import (
    PublicFormatPlan,
    PublicPoolError,
    PublicSourceGroup,
)
from evaluate.multiformat_schema import (
    JsonValue,
    integer_value,
    object_value,
    sha256_file,
    sha256_value,
    string_value,
)
from evaluate.multiformat_strict_json import StrictJsonError, read_strict_object

_LEGACY_FORMATS = {
    DocumentFormat.DOC,
    DocumentFormat.XLS,
    DocumentFormat.PPT,
}


class LegacyBinaryPoolError(Exception):
    pass


def load_legacy_binary_plans(
    config_path: Path,
    public_config_path: Path,
) -> tuple[PublicFormatPlan, ...]:
    try:
        values = read_strict_object(config_path)
        require_keys(
            values,
            {"schema_version", "source_catalog_sha256", "formats"},
            "legacy.binary.config",
        )
        if integer_value(values, "schema_version") != 1 or sha256_value(
            values, "source_catalog_sha256"
        ) != sha256_file(public_config_path):
            raise LegacyBinaryPoolError("legacy binary source catalog binding differs")
        public = {
            plan.document_format: plan
            for plan in load_public_pool_plans(public_config_path)
        }
        result = tuple(
            _format_plan(name, value, public)
            for name, value in object_value(values, "formats").items()
        )
        if not result or len({plan.document_format for plan in result}) != len(result):
            raise LegacyBinaryPoolError("legacy binary format set differs")
        return tuple(sorted(result, key=lambda plan: plan.document_format.value))
    except LegacyBinaryPoolError:
        raise
    except (OSError, PublicPoolError, StrictJsonError, ValueError) as error:
        raise LegacyBinaryPoolError("legacy binary config is invalid") from error


def _format_plan(
    format_name: str,
    value: JsonValue,
    public: dict[DocumentFormat, PublicFormatPlan],
) -> PublicFormatPlan:
    if not isinstance(value, dict):
        raise LegacyBinaryPoolError("legacy binary format config is invalid")
    try:
        document_format = DocumentFormat(format_name)
    except ValueError as error:
        raise LegacyBinaryPoolError("legacy binary format is unsupported") from error
    if document_format not in _LEGACY_FORMATS or document_format not in public:
        raise LegacyBinaryPoolError("legacy binary format is unsupported")
    require_keys(
        value,
        {"expected_count", "groups"},
        "legacy.binary.config.format",
    )
    public_groups = {group.producer: group for group in public[document_format].groups}
    groups = tuple(
        _group(item, public_groups)
        for item in object_list(
            value,
            "groups",
            "legacy.binary.config.groups",
        )
    )
    expected = integer_value(value, "expected_count")
    if (
        expected <= 0
        or len(groups) < 2
        or len({group.producer for group in groups}) != len(groups)
        or sum(group.quota for group in groups) != expected
    ):
        raise LegacyBinaryPoolError("legacy binary group quota differs")
    return PublicFormatPlan(document_format, expected, groups)


def _group(
    value: dict[str, JsonValue],
    public: dict[str, PublicSourceGroup],
) -> PublicSourceGroup:
    require_keys(value, {"producer", "quota"}, "legacy.binary.config.group")
    producer = string_value(value, "producer")
    quota = integer_value(value, "quota")
    source = public.get(producer)
    if source is None or quota <= 0:
        raise LegacyBinaryPoolError("legacy binary source group differs")
    return replace(source, quota=quota)
