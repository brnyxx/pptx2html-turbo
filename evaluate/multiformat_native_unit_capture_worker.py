from __future__ import annotations

from dataclasses import dataclass, replace
from pathlib import Path
from typing import Protocol

from evaluate.multiformat_native_unit_runtime import capture_native_observation
from evaluate.multiformat_native_unit_cache import NativeObservationCache
from evaluate.multiformat_native_unit_capture_gates import (
    NativeTwoWorkerGate,
    run_two_worker_gate,
    select_two_worker_requests,
)
from evaluate.multiformat_native_unit_capture_phase import (
    NativeCaptureItem,
    run_capture_phase,
)
from evaluate.multiformat_native_unit_tool_probe import prepare_capture_tools
from evaluate.multiformat_native_unit_types import (
    NativeCaptureTools,
    NativeObservation,
    NativeProcessRunner,
    NativeUnitRequest,
    NativeUnitRuntime,
    NativeUnitSource,
)
from evaluate.multiformat_public_pool_types import ValidatedPublicPoolSource
from evaluate.multiformat_reference_routing import RoutingIdentity
from evaluate.multiformat_schema import sha256_file


class NativeCaptureRuntimeInputs(Protocol):
    @property
    def contract(self) -> Path: ...

    @property
    def public_pool_manifest(self) -> Path: ...

    @property
    def font_manifest(self) -> Path: ...

    @property
    def libreoffice(self) -> Path: ...

    @property
    def pdfinfo(self) -> Path: ...

    @property
    def workers(self) -> int: ...

    @property
    def cache_dir(self) -> Path | None: ...


@dataclass(frozen=True, slots=True)
class NativeCaptureResult:
    observations: tuple[NativeObservation, ...]
    tools: NativeCaptureTools
    two_worker_gate: NativeTwoWorkerGate


def capture_all(
    inputs: NativeCaptureRuntimeInputs,
    staging: Path,
    sources: tuple[ValidatedPublicPoolSource, ...],
    routing: RoutingIdentity,
    nonces: tuple[str, ...],
    runner: NativeProcessRunner,
    font_manifest_sha256: str,
    font_environment_sha256: str,
) -> NativeCaptureResult:
    runtime = NativeUnitRuntime(
        inputs.libreoffice,
        inputs.pdfinfo,
        inputs.font_manifest,
        routing,
    )
    requests: list[NativeUnitRequest] = []
    source_hashes: list[str] = []
    nonce_index = 0
    root = inputs.public_pool_manifest.parent
    for source in sources:
        for run in (1, 2):
            document_format = source.document_format
            source_id = source.source_id
            relative_path = source.relative_path
            requests.append(
                NativeUnitRequest(
                    NativeUnitSource(
                        source_id,
                        document_format,
                        root / relative_path,
                        relative_path,
                    ),
                    runtime,
                    staging
                    / "observations"
                    / document_format.value
                    / source_id
                    / f"run-{run}",
                    run,
                    nonces[nonce_index],
                )
            )
            source_hashes.append(source.source_sha256)
            nonce_index += 1
    tools = prepare_capture_tools(staging.parent, requests[0], runner)
    prepared_runtime = replace(runtime, tools=tools)
    requests = [replace(request, runtime=prepared_runtime) for request in requests]
    items = tuple(
        NativeCaptureItem(request, source_sha256)
        for request, source_sha256 in zip(requests, source_hashes, strict=True)
    )
    cache = (
        NativeObservationCache(
            inputs.cache_dir,
            sha256_file(inputs.contract),
            font_manifest_sha256,
            font_environment_sha256,
            tools,
        )
        if inputs.cache_dir is not None
        else None
    )

    def capture_request(
        request: NativeUnitRequest,
        source_sha256: str,
    ) -> NativeObservation:
        if cache is None:
            return capture_native_observation(request, runner)
        return cache.capture(request, source_sha256, runner)

    preflight = tuple(item for item in items if item.request.run == 1)
    observations = list(run_capture_phase(preflight, 1, capture_request))
    stress_requests = select_two_worker_requests(tuple(requests))
    gate = run_two_worker_gate(stress_requests, runner)
    stress_paths = {request.observation_dir for request in stress_requests}
    capture_items = tuple(
        item
        for item in items
        if item.request.run == 2 and item.request.observation_dir not in stress_paths
    )
    observations.extend(
        run_capture_phase(capture_items, inputs.workers, capture_request)
    )
    observations.extend(gate.observations)
    return NativeCaptureResult(
        tuple(
            sorted(
                observations,
                key=lambda item: (
                    item.source.document_format.value,
                    item.source.source_id,
                    item.run,
                ),
            )
        ),
        tools,
        gate,
    )
