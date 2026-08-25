from __future__ import annotations

from collections.abc import Callable
from pathlib import Path

from evaluate.jcs import canonicalize
from evaluate.multiformat_corpus_items import object_list, require_keys
from evaluate.multiformat_corpus_types import CorpusError, DocumentFormat
from evaluate.multiformat_native_unit_execution_validation import (
    NativeExecutionBindings,
    validate_execution_record,
)
from evaluate.multiformat_native_unit_stable_validation import stable_bytes
from evaluate.multiformat_native_unit_tool_validation import LockedTool
from evaluate.multiformat_native_unit_types import NativeUnitError, NativeUnitFailure
from evaluate.multiformat_public_pool_types import ValidatedPublicPoolSource
from evaluate.multiformat_schema import (
    JsonValue,
    integer_value,
    object_value,
    sha256_value,
    string_value,
)
from evaluate.multiformat_strict_json import MAX_JSON_BYTES, parse_strict_object_bytes

PdfCountValidator = Callable[[Path, LockedTool, Path, int], None]
_MAX_PDF_BYTES = 64 * 1024 * 1024
_MAX_PDFINFO_BYTES = 1024 * 1024


def validate_scoped_source(
    root: Path,
    pdfinfo: Path,
    pdfinfo_tool: LockedTool,
    expected: ValidatedPublicPoolSource,
    source: dict[str, JsonValue],
    nonces: set[str],
    execution_bindings: NativeExecutionBindings,
    pdf_count_validator: PdfCountValidator,
) -> tuple[int, set[str]]:
    try:
        return validate_source_record(
            root,
            pdfinfo,
            pdfinfo_tool,
            expected,
            source,
            nonces,
            execution_bindings,
            pdf_count_validator,
        )
    except NativeUnitError as error:
        if (
            error.document_format is expected.document_format
            and error.source_id == expected.source_id
        ):
            raise
        raise NativeUnitError(
            error.failure,
            expected.document_format,
            expected.source_id,
            error.detail,
        ) from error
    except (CorpusError, OSError, TypeError, ValueError) as error:
        raise source_failure(
            expected,
            "inventory source validation failed",
        ) from error


def source_failure(
    source: ValidatedPublicPoolSource,
    detail: str,
) -> NativeUnitError:
    return NativeUnitError(
        NativeUnitFailure.OUTPUT_INVALID,
        source.document_format,
        source.source_id,
        detail,
    )


def validate_source_record(
    root: Path,
    pdfinfo: Path,
    pdfinfo_tool: LockedTool,
    expected: ValidatedPublicPoolSource,
    source: dict[str, JsonValue],
    nonces: set[str],
    execution_bindings: NativeExecutionBindings,
    pdf_count_validator: PdfCountValidator,
) -> tuple[int, set[str]]:
    require_keys(
        source,
        {"id", "format", "path", "sha256", "unit_count", "observations"},
        "native.inventory.source",
    )
    values = (
        string_value(source, "format"),
        string_value(source, "id"),
        string_value(source, "path"),
        sha256_value(source, "sha256"),
    )
    expected_values = (
        expected.document_format.value,
        expected.source_id,
        expected.relative_path,
        expected.source_sha256,
    )
    count = integer_value(source, "unit_count")
    if values != expected_values or count <= 0:
        raise _failure("inventory source binding differs")
    observations = object_list(source, "observations", "native.inventory.observations")
    if len(observations) != 2:
        raise _failure("inventory observation count differs")
    bindings: set[str] = set()
    for run, observation in enumerate(observations, start=1):
        current = _validate_observation(
            root,
            pdfinfo,
            pdfinfo_tool,
            values,
            count,
            run,
            observation,
            nonces,
            execution_bindings,
            pdf_count_validator,
        )
        if bindings & current:
            raise _failure("inventory evidence path is duplicated")
        bindings.update(current)
    return count, bindings


def _validate_observation(
    root: Path,
    pdfinfo: Path,
    pdfinfo_tool: LockedTool,
    source_values: tuple[str, str, str, str],
    count: int,
    run: int,
    observation: dict[str, JsonValue],
    nonces: set[str],
    execution_bindings: NativeExecutionBindings,
    pdf_count_validator: PdfCountValidator,
) -> set[str]:
    require_keys(
        observation,
        {"run", "workspace_nonce", "path", "execution", "reference_pdf", "pdfinfo"},
        "native.inventory.observation",
    )
    document_format, source_id, relative_path, source_sha256 = source_values
    base = f"observations/{document_format}/{source_id}/run-{run}"
    nonce = string_value(observation, "workspace_nonce")
    if (
        integer_value(observation, "run") != run
        or string_value(observation, "path") != base
        or not _valid_nonce(nonce)
        or nonce in nonces
    ):
        raise _failure("inventory observation identity differs")
    nonces.add(nonce)
    result: set[str] = set()
    parent_bindings: dict[str, dict[str, JsonValue]] = {}
    contents: dict[str, bytes] = {}
    for field, name in (
        ("execution", "execution.json"),
        ("reference_pdf", "reference.pdf"),
        ("pdfinfo", "pdfinfo.txt"),
    ):
        binding = object_value(observation, field)
        require_keys(binding, {"path", "sha256"}, f"observation.{field}")
        relative = string_value(binding, "path")
        if relative != f"{base}/{name}" or relative in result:
            raise _failure("inventory evidence path differs")
        path = root / relative
        maximum = {
            "execution": MAX_JSON_BYTES,
            "reference_pdf": _MAX_PDF_BYTES,
            "pdfinfo": _MAX_PDFINFO_BYTES,
        }[field]
        file_state, content = stable_bytes(
            path,
            executable=False,
            maximum=maximum,
        )
        if file_state[-1] != sha256_value(binding, "sha256"):
            raise _failure("inventory evidence hash differs")
        parent_bindings[field] = binding
        contents[field] = content
        result.add(relative)
    execution_bytes = contents["execution"]
    execution = parse_strict_object_bytes(execution_bytes)
    if execution_bytes != canonicalize(execution) + b"\n":
        raise _failure("execution record is not canonical JCS")
    validate_execution_record(
        execution,
        DocumentFormat(document_format),
        source_id,
        relative_path,
        source_sha256,
        count,
        run,
        nonce,
        parent_bindings,
        execution_bindings,
    )
    pdf_count_validator(
        pdfinfo,
        pdfinfo_tool,
        root / f"{base}/reference.pdf",
        count,
    )
    return result


def _valid_nonce(value: str) -> bool:
    return len(value) == 64 and all(
        character in "0123456789abcdef" for character in value
    )


def _failure(detail: str) -> NativeUnitError:
    return NativeUnitError(NativeUnitFailure.OUTPUT_INVALID, None, None, detail)
