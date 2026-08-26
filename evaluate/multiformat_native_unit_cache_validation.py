from __future__ import annotations

import hashlib
from dataclasses import dataclass
from pathlib import Path

from evaluate.jcs import canonicalize
from evaluate.multiformat_corpus_items import require_keys
from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_native_unit_files import (
    MAX_LOG_BYTES,
    MAX_PDF_BYTES,
    fail,
)
from evaluate.multiformat_native_unit_process import pages
from evaluate.multiformat_native_unit_stable_validation import (
    StableFile,
    open_stable_directory,
    stable_bytes_at,
    stable_file_at,
)
from evaluate.multiformat_native_unit_tree_validation import validate_inventory_tree
from evaluate.multiformat_native_unit_types import (
    NativeUnitFailure,
    NativeUnitRequest,
)
from evaluate.multiformat_schema import (
    JsonValue,
    integer_value,
    object_value,
    sha256_value,
    string_value,
)
from evaluate.multiformat_strict_json import (
    MAX_JSON_BYTES,
    parse_strict_object_bytes,
)


@dataclass(frozen=True, slots=True)
class LoadedCacheEntry:
    contents: dict[str, bytes]
    nonce: str
    unit_count: int


def load_cache_entry(
    entry: Path,
    key: str,
    key_value: dict[str, JsonValue],
    request: NativeUnitRequest,
) -> LoadedCacheEntry:
    maximums = {
        "cache.json": MAX_JSON_BYTES,
        "execution.json": MAX_JSON_BYTES,
        "reference.pdf": MAX_PDF_BYTES,
        "pdfinfo.txt": MAX_LOG_BYTES,
    }
    with open_stable_directory(entry) as descriptor:
        states: dict[str, StableFile] = {}
        contents: dict[str, bytes] = {}
        for name, maximum in maximums.items():
            state, content = stable_bytes_at(
                descriptor,
                name,
                executable=False,
                maximum=maximum,
            )
            states[name] = state
            contents[name] = content
        metadata = _validate_metadata(contents, key, key_value, request)
        for name, state in states.items():
            if (
                stable_file_at(
                    descriptor,
                    name,
                    executable=False,
                    maximum=maximums[name],
                )
                != state
            ):
                raise fail(
                    request,
                    NativeUnitFailure.OUTPUT_INVALID,
                    "observation cache changed during validation",
                )
        validate_inventory_tree(descriptor, tuple(sorted(states.items())))
    return LoadedCacheEntry(
        contents,
        string_value(metadata, "workspace_nonce"),
        integer_value(metadata, "unit_count"),
    )


def _validate_metadata(
    contents: dict[str, bytes],
    key: str,
    key_value: dict[str, JsonValue],
    request: NativeUnitRequest,
) -> dict[str, JsonValue]:
    metadata = parse_strict_object_bytes(contents["cache.json"])
    if contents["cache.json"] != canonicalize(metadata) + b"\n":
        raise _failure(request, "observation cache metadata is not canonical")
    require_keys(
        metadata,
        {
            "schema_version",
            "cache_key",
            "key",
            "workspace_nonce",
            "unit_count",
            "files",
        },
        "native.observation.cache",
    )
    if (
        integer_value(metadata, "schema_version") != 1
        or string_value(metadata, "cache_key") != key
        or object_value(metadata, "key") != key_value
    ):
        raise _failure(request, "observation cache key differs")
    files = object_value(metadata, "files")
    require_keys(
        files,
        {"execution.json", "reference.pdf", "pdfinfo.txt"},
        "native.observation.cache.files",
    )
    for name in files:
        binding = object_value(files, name)
        require_keys(binding, {"sha256", "size"}, "native.observation.cache.file")
        content = contents[name]
        if sha256_value(binding, "sha256") != hashlib.sha256(
            content
        ).hexdigest() or integer_value(binding, "size") != len(content):
            raise _failure(request, "observation cache file binding differs")
    reference = contents["reference.pdf"]
    count = integer_value(metadata, "unit_count")
    if (
        not reference.startswith(b"%PDF-")
        or pages(contents["pdfinfo.txt"], request) != count
    ):
        raise _failure(request, "observation cache evidence is invalid")
    execution = parse_strict_object_bytes(contents["execution.json"])
    if contents["execution.json"] != canonicalize(execution) + b"\n":
        raise _failure(request, "cached execution is not canonical")
    _validate_execution(execution, metadata, key_value, contents, request)
    return metadata


def _validate_execution(
    execution: dict[str, JsonValue],
    metadata: dict[str, JsonValue],
    key_value: dict[str, JsonValue],
    contents: dict[str, bytes],
    request: NativeUnitRequest,
) -> None:
    source = object_value(execution, "source")
    key_source = object_value(key_value, "source")
    tools = object_value(execution, "tools")
    key_tools = object_value(key_value, "tools")
    expected_tools = {"pdfinfo": object_value(key_tools, "pdfinfo")}
    if request.source.document_format is not DocumentFormat.PDF:
        expected_tools["libreoffice"] = object_value(key_tools, "libreoffice")
    evidence = object_value(execution, "evidence")
    environment = object_value(execution, "environment")
    font = object_value(key_value, "font")
    invalid_font = (
        request.source.document_format is not DocumentFormat.PDF
        and sha256_value(environment, "font_environment_sha256")
        != sha256_value(font, "environment_sha256")
    )
    if (
        integer_value(execution, "schema_version") != 2
        or string_value(source, "id") != request.source.source_id
        or string_value(source, "format") != request.source.document_format.value
        or string_value(source, "path") != request.source.relative_path
        or sha256_value(source, "sha256") != sha256_value(key_source, "sha256")
        or integer_value(execution, "run") != request.run
        or string_value(execution, "workspace_nonce")
        != string_value(metadata, "workspace_nonce")
        or integer_value(execution, "unit_count")
        != integer_value(metadata, "unit_count")
        or sha256_value(execution, "routing_sha256")
        != string_value(key_value, "routing_sha256")
        or tools != expected_tools
        or invalid_font
        or sha256_value(object_value(evidence, "reference_pdf"), "sha256")
        != hashlib.sha256(contents["reference.pdf"]).hexdigest()
        or sha256_value(object_value(evidence, "pdfinfo"), "sha256")
        != hashlib.sha256(contents["pdfinfo.txt"]).hexdigest()
    ):
        raise _failure(request, "observation cache execution binding differs")


def _failure(request: NativeUnitRequest, detail: str):
    return fail(request, NativeUnitFailure.OUTPUT_INVALID, detail)
