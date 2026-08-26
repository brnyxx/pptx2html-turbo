from __future__ import annotations

import hashlib
import os
from dataclasses import replace
from pathlib import Path

from evaluate.jcs import canonicalize
from evaluate.multiformat_native_unit_cache_validation import load_cache_entry
from evaluate.multiformat_native_unit_files import (
    MAX_LOG_BYTES,
    MAX_PDF_BYTES,
    fail,
    stable_bytes,
)
from evaluate.multiformat_native_unit_io import write_new
from evaluate.multiformat_native_unit_runtime import capture_native_observation
from evaluate.multiformat_native_unit_types import (
    NativeCaptureTool,
    NativeCaptureTools,
    NativeObservation,
    NativeProcessRunner,
    NativeUnitFailure,
    NativeUnitRequest,
)
from evaluate.multiformat_schema import (
    JsonValue,
)
from evaluate.multiformat_snapshot_publish import (
    SnapshotPublishError,
    SnapshotPublishFailure,
    publish_snapshot,
)
from evaluate.multiformat_strict_json import MAX_JSON_BYTES


class NativeObservationCache:
    def __init__(
        self,
        root: Path,
        contract_sha256: str,
        font_manifest_sha256: str,
        font_environment_sha256: str,
        tools: NativeCaptureTools,
    ) -> None:
        self._root = root
        self._contract_sha256 = contract_sha256
        self._font_manifest_sha256 = font_manifest_sha256
        self._font_environment_sha256 = font_environment_sha256
        self._tools = tools

    def capture(
        self,
        request: NativeUnitRequest,
        source_sha256: str,
        runner: NativeProcessRunner,
    ) -> NativeObservation:
        key_value = self._key_value(request, source_sha256)
        key_bytes = canonicalize(key_value)
        key = hashlib.sha256(key_bytes).hexdigest()
        nonce = hashlib.sha256(b"native-observation-cache-v1\0" + key_bytes).hexdigest()
        prepared = replace(request, nonce=nonce)
        entry = self._root / "v1" / key[:2] / key
        if os.path.lexists(entry):
            return self._materialize(entry, key, key_value, prepared)
        observation = capture_native_observation(prepared, runner)
        try:
            publish_snapshot(
                entry,
                lambda staging: self._store(
                    staging,
                    key,
                    key_value,
                    observation,
                    prepared,
                ),
                lock_namespace="native-cache",
            )
        except SnapshotPublishError as error:
            if error.failure is not SnapshotPublishFailure.DESTINATION_EXISTS:
                raise fail(
                    prepared,
                    NativeUnitFailure.OUTPUT_INVALID,
                    "observation cache publication failed",
                ) from error
            _ = load_cache_entry(entry, key, key_value, prepared)
        return observation

    def _store(
        self,
        staging: Path,
        key: str,
        key_value: dict[str, JsonValue],
        observation: NativeObservation,
        request: NativeUnitRequest,
    ) -> None:
        contents: dict[str, bytes] = {}
        for name, path, maximum in (
            ("execution.json", observation.execution_path, MAX_JSON_BYTES),
            ("reference.pdf", observation.reference_pdf_path, MAX_PDF_BYTES),
            ("pdfinfo.txt", observation.pdfinfo_path, MAX_LOG_BYTES),
        ):
            _state, content = stable_bytes(
                path,
                request,
                NativeUnitFailure.OUTPUT_INVALID,
                maximum,
            )
            contents[name] = content
            _ = write_new(staging / name, content, request)
        files: dict[str, JsonValue] = {
            name: {
                "sha256": hashlib.sha256(content).hexdigest(),
                "size": len(content),
            }
            for name, content in contents.items()
        }
        metadata: dict[str, JsonValue] = {
            "schema_version": 1,
            "cache_key": key,
            "key": key_value,
            "workspace_nonce": observation.workspace_nonce,
            "unit_count": observation.unit_count,
            "files": files,
        }
        _ = write_new(staging / "cache.json", canonicalize(metadata) + b"\n", request)
        _ = load_cache_entry(staging, key, key_value, request)

    def _materialize(
        self,
        entry: Path,
        key: str,
        key_value: dict[str, JsonValue],
        request: NativeUnitRequest,
    ) -> NativeObservation:
        loaded = load_cache_entry(entry, key, key_value, request)

        def writer(staging: Path) -> None:
            for name in ("execution.json", "reference.pdf", "pdfinfo.txt"):
                content = loaded.contents[name]
                _ = write_new(staging / name, content, request)

        try:
            publish_snapshot(
                request.observation_dir,
                writer,
                lock_namespace="native-unit",
            )
        except SnapshotPublishError as error:
            raise fail(
                request,
                NativeUnitFailure.OUTPUT_INVALID,
                "cached observation publication failed",
            ) from error
        return NativeObservation(
            request.source,
            request.run,
            loaded.nonce,
            loaded.unit_count,
            request.observation_dir,
            request.observation_dir / "execution.json",
            request.observation_dir / "reference.pdf",
            request.observation_dir / "pdfinfo.txt",
            hashlib.sha256(loaded.contents["execution.json"]).hexdigest(),
            hashlib.sha256(loaded.contents["reference.pdf"]).hexdigest(),
            hashlib.sha256(loaded.contents["pdfinfo.txt"]).hexdigest(),
        )

    def _key_value(
        self,
        request: NativeUnitRequest,
        source_sha256: str,
    ) -> dict[str, JsonValue]:
        return {
            "schema_version": 1,
            "contract_sha256": self._contract_sha256,
            "source": {
                "id": request.source.source_id,
                "format": request.source.document_format.value,
                "path": request.source.relative_path,
                "sha256": source_sha256,
            },
            "run": request.run,
            "routing_sha256": request.runtime.routing.sha256,
            "font": {
                "manifest_sha256": self._font_manifest_sha256,
                "environment_sha256": self._font_environment_sha256,
            },
            "tools": {
                "libreoffice": self._tool_value(
                    request.runtime.soffice,
                    self._tools.libreoffice,
                ),
                "pdfinfo": self._tool_value(
                    request.runtime.pdfinfo,
                    self._tools.pdfinfo,
                ),
            },
        }

    @staticmethod
    def _tool_value(path: Path, tool: NativeCaptureTool) -> dict[str, JsonValue]:
        return {
            "name": path.name,
            "sha256": tool.identity.sha256,
            "version": tool.version,
        }
