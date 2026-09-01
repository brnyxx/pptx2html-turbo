from __future__ import annotations

import threading
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass

from evaluate.multiformat_candidate_process import (
    CandidateProcessError,
    CandidateProcessFailure,
)
from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_native_unit_runtime import capture_native_observation
from evaluate.multiformat_native_unit_types import (
    NativeObservation,
    NativeProcessRequest,
    NativeProcessRunner,
    NativeUnitError,
    NativeUnitFailure,
    NativeUnitRequest,
)
from evaluate.multiformat_schema import JsonValue

STRESS_WORKERS = 2
STRESS_BARRIER_TIMEOUT_SECONDS = 120


@dataclass(frozen=True, slots=True)
class NativeTwoWorkerGate:
    observations: tuple[NativeObservation, NativeObservation]


def two_worker_gate_value(
    gate: NativeTwoWorkerGate,
) -> dict[str, JsonValue]:
    observations: list[JsonValue] = [
        {
            "format": observation.source.document_format.value,
            "id": observation.source.source_id,
            "run": observation.run,
            "execution_sha256": observation.execution_sha256,
        }
        for observation in gate.observations
    ]
    return {
        "status": "PASSED",
        "worker_count": STRESS_WORKERS,
        "coordinator": "barrier-before-libreoffice-v1",
        "observations": observations,
    }


def convertibility_preflight_value(
    observations: tuple[NativeObservation, ...],
) -> dict[str, JsonValue]:
    run_one = tuple(observation for observation in observations if observation.run == 1)
    return {
        "status": "PASSED",
        "worker_count": 1,
        "observation_run": 1,
        "source_count": len(run_one),
    }


def derive_two_worker_gate(
    observations: tuple[NativeObservation, ...],
) -> NativeTwoWorkerGate:
    selected = sorted(
        (
            observation
            for observation in observations
            if observation.run == 2
            and observation.source.document_format is not DocumentFormat.PDF
        ),
        key=lambda observation: (
            observation.source.document_format.value,
            observation.source.source_id,
        ),
    )[:2]
    if len(selected) != STRESS_WORKERS:
        raise _failure("two-worker gate selection differs")
    return NativeTwoWorkerGate((selected[0], selected[1]))


def select_two_worker_requests(
    requests: tuple[NativeUnitRequest, ...],
) -> tuple[NativeUnitRequest, NativeUnitRequest]:
    candidates = sorted(
        (
            request
            for request in requests
            if request.run == 2
            and request.source.document_format is not DocumentFormat.PDF
        ),
        key=lambda request: (
            request.source.document_format.value,
            request.source.source_id,
        ),
    )
    if len(candidates) < STRESS_WORKERS:
        raise _failure("two-worker stress candidates are missing")
    return candidates[0], candidates[1]


def run_two_worker_gate(
    requests: tuple[NativeUnitRequest, NativeUnitRequest],
    runner: NativeProcessRunner,
    *,
    barrier: threading.Barrier | None = None,
) -> NativeTwoWorkerGate:
    coordinator = barrier or threading.Barrier(STRESS_WORKERS)
    coordinated = _BarrierRunner(runner, coordinator)
    with ThreadPoolExecutor(max_workers=STRESS_WORKERS) as executor:
        futures = tuple(
            executor.submit(capture_native_observation, request, coordinated)
            for request in requests
        )
        observations = tuple(future.result() for future in futures)
    return NativeTwoWorkerGate((observations[0], observations[1]))


class _BarrierRunner:
    def __init__(
        self,
        runner: NativeProcessRunner,
        barrier: threading.Barrier,
    ) -> None:
        self._runner = runner
        self._barrier = barrier

    def __call__(self, request: NativeProcessRequest) -> int:
        if "--convert-to" in request.command:
            try:
                _ = self._barrier.wait(timeout=STRESS_BARRIER_TIMEOUT_SECONDS)
            except threading.BrokenBarrierError as error:
                raise CandidateProcessError(
                    CandidateProcessFailure.PIPES_UNAVAILABLE
                ) from error
        return self._runner(request)


def _failure(detail: str) -> NativeUnitError:
    return NativeUnitError(NativeUnitFailure.OUTPUT_INVALID, None, None, detail)
