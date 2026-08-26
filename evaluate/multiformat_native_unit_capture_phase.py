from __future__ import annotations

from collections.abc import Callable
from concurrent.futures import ThreadPoolExecutor, as_completed
from dataclasses import dataclass

from evaluate.multiformat_native_unit_types import (
    NativeObservation,
    NativeUnitError,
    NativeUnitRequest,
)


@dataclass(frozen=True, slots=True)
class NativeCaptureItem:
    request: NativeUnitRequest
    source_sha256: str


def run_capture_phase(
    items: tuple[NativeCaptureItem, ...],
    workers: int,
    capture: Callable[[NativeUnitRequest, str], NativeObservation],
) -> tuple[NativeObservation, ...]:
    with ThreadPoolExecutor(max_workers=workers) as executor:
        futures = {
            executor.submit(capture, item.request, item.source_sha256): item.request
            for item in items
        }
        observations: list[NativeObservation] = []
        failures: list[tuple[tuple[str, str, int], NativeUnitError]] = []
        for future in as_completed(futures):
            request = futures[future]
            try:
                observations.append(future.result())
            except NativeUnitError as error:
                failures.append(
                    (
                        (
                            request.source.document_format.value,
                            request.source.source_id,
                            request.run,
                        ),
                        error,
                    )
                )
    if failures:
        raise sorted(failures, key=lambda item: item[0])[0][1]
    return tuple(observations)
