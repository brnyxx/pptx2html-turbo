from __future__ import annotations

import platform
import secrets
from collections import Counter
from collections.abc import Callable
from concurrent.futures import ThreadPoolExecutor, as_completed
from dataclasses import dataclass
from pathlib import Path

from evaluate.jcs import canonicalize
from evaluate.multiformat_corpus_types import CorpusError, DocumentFormat
from evaluate.multiformat_font_snapshot import (
    FontSnapshotError,
    validate_font_snapshot,
)
from evaluate.multiformat_native_unit_manifest import (
    NativeManifestInputs,
    build_native_unit_manifest,
)
from evaluate.multiformat_native_unit_process import run_native_process
from evaluate.multiformat_native_unit_runtime import capture_native_observation
from evaluate.multiformat_native_unit_types import (
    NativeObservation,
    NativeProcessRunner,
    NativeUnitError,
    NativeUnitFailure,
    NativeUnitRequest,
    NativeUnitRuntime,
    NativeUnitSource,
)
from evaluate.multiformat_native_unit_validation import (
    NativeUnitInventorySummary,
    NativeUnitValidationInputs,
    validate_native_unit_inventory,
)
from evaluate.multiformat_public_pool import load_validated_public_pool_sources
from evaluate.multiformat_public_pool_types import (
    PublicPoolError,
    ValidatedPublicPoolSource,
)
from evaluate.multiformat_reference_routing import (
    RoutingError,
    RoutingIdentity,
    load_reference_routing,
)
from evaluate.multiformat_snapshot_publish import SnapshotPublishError, publish_snapshot
from evaluate.multiformat_strict_json import read_strict_object

NonceFactory = Callable[[], str]


@dataclass(frozen=True, slots=True)
class NativeUnitCaptureInputs:
    contract: Path
    public_config: Path
    public_pool_manifest: Path
    routing: Path
    font_manifest: Path
    libreoffice: Path
    pdfinfo: Path
    output_dir: Path
    workers: int


def generate_native_nonce() -> str:
    return secrets.token_hex(32)


def capture_native_unit_inventory(
    inputs: NativeUnitCaptureInputs,
    *,
    runner: NativeProcessRunner = run_native_process,
    nonce_factory: NonceFactory = generate_native_nonce,
) -> NativeUnitInventorySummary:
    operating_system, architecture = _platform()
    if type(inputs.workers) is not int or not 1 <= inputs.workers <= 8:
        raise _failure("worker count is invalid")
    try:
        _ = read_strict_object(inputs.contract)
        sources = load_validated_public_pool_sources(
            inputs.public_config,
            inputs.public_pool_manifest,
        )
        counts = Counter(source.document_format for source in sources)
        if len(sources) != 525 or counts != Counter(
            {document_format: 75 for document_format in DocumentFormat}
        ):
            raise _failure("public pool must contain seven by 75 sources")
        routing = load_reference_routing(inputs.routing)
        font = validate_font_snapshot(
            inputs.font_manifest,
            inputs.font_manifest.parent,
        )
        nonces = tuple(nonce_factory() for _ in range(1_050))
        if len(set(nonces)) != 1_050 or any(
            not _valid_nonce(nonce) for nonce in nonces
        ):
            raise _failure("capture nonces are invalid or duplicated")
        summaries: list[NativeUnitInventorySummary] = []

        def writer(staging: Path) -> None:
            observations = _capture_all(
                inputs,
                staging,
                sources,
                routing,
                nonces,
                runner,
            )
            values = build_native_unit_manifest(
                NativeManifestInputs(
                    inputs.contract,
                    inputs.public_config,
                    inputs.public_pool_manifest,
                    inputs.workers,
                    operating_system,
                    architecture,
                    font.manifest_sha256,
                    font.environment_sha256,
                ),
                observations,
            )
            _ = (staging / "native-unit-inventory.json").write_bytes(
                canonicalize(values) + b"\n"
            )
            summaries.append(
                validate_native_unit_inventory(_validation_inputs(inputs, staging))
            )

        publish_snapshot(inputs.output_dir, writer, lock_namespace="snapshot")
        if len(summaries) != 1:
            raise _failure("capture summary is missing")
        return summaries[0]
    except (
        CorpusError,
        FontSnapshotError,
        PublicPoolError,
        RoutingError,
        SnapshotPublishError,
        OSError,
        TypeError,
        ValueError,
    ) as error:
        raise _failure("native inventory capture failed") from error


def _capture_all(
    inputs: NativeUnitCaptureInputs,
    staging: Path,
    sources: tuple[ValidatedPublicPoolSource, ...],
    routing: RoutingIdentity,
    nonces: tuple[str, ...],
    runner: NativeProcessRunner,
) -> tuple[NativeObservation, ...]:
    runtime = NativeUnitRuntime(
        inputs.libreoffice,
        inputs.pdfinfo,
        inputs.font_manifest,
        routing,
    )
    requests: list[NativeUnitRequest] = []
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
            nonce_index += 1
    with ThreadPoolExecutor(max_workers=inputs.workers) as executor:
        futures = [
            executor.submit(capture_native_observation, request, runner)
            for request in requests
        ]
        observations = [future.result() for future in as_completed(futures)]
    return tuple(
        sorted(
            observations,
            key=lambda item: (
                item.source.document_format.value,
                item.source.source_id,
                item.run,
            ),
        )
    )


def _validation_inputs(
    inputs: NativeUnitCaptureInputs,
    root: Path,
) -> NativeUnitValidationInputs:
    return NativeUnitValidationInputs(
        inputs.contract,
        inputs.public_config,
        inputs.public_pool_manifest,
        inputs.routing,
        inputs.font_manifest,
        inputs.libreoffice,
        inputs.pdfinfo,
        root,
    )


def _platform() -> tuple[str, str]:
    operating_system = {"darwin": "macos", "linux": "linux"}.get(
        platform.system().lower()
    )
    architecture = {
        "aarch64": "arm64",
        "arm64": "arm64",
        "amd64": "x86_64",
        "x86_64": "x86_64",
    }.get(platform.machine().lower())
    if operating_system is None or architecture is None:
        raise NativeUnitError(
            NativeUnitFailure.UNSUPPORTED_PLATFORM,
            None,
            None,
            "unsupported platform or architecture",
        )
    return operating_system, architecture


def _valid_nonce(value: str) -> bool:
    return len(value) == 64 and all(
        character in "0123456789abcdef" for character in value
    )


def _failure(detail: str) -> NativeUnitError:
    return NativeUnitError(NativeUnitFailure.OUTPUT_INVALID, None, None, detail)
