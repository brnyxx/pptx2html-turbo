from __future__ import annotations

from evaluate.multiformat_corpus_items import object_list, require_keys
from evaluate.multiformat_native_unit_types import NativeUnitError, NativeUnitFailure
from evaluate.multiformat_schema import (
    JsonValue,
    integer_value,
    object_value,
    sha256_value,
    string_value,
)


def validate_two_worker_gate(
    manifest: dict[str, JsonValue],
    sources: list[dict[str, JsonValue]],
) -> None:
    gate = object_value(manifest, "two_worker_gate")
    require_keys(
        gate,
        {"status", "worker_count", "coordinator", "observations"},
        "native.inventory.two_worker_gate",
    )
    if (
        string_value(gate, "status") != "PASSED"
        or integer_value(gate, "worker_count") != 2
        or string_value(gate, "coordinator") != "barrier-before-libreoffice-v1"
    ):
        raise _failure("two-worker gate metadata differs")
    gate_observations = object_list(
        gate,
        "observations",
        "native.inventory.two_worker_gate.observations",
    )
    candidates = sorted(
        (source for source in sources if string_value(source, "format") != "pdf"),
        key=lambda source: (
            string_value(source, "format"),
            string_value(source, "id"),
        ),
    )[:2]
    if len(gate_observations) != 2 or len(candidates) != 2:
        raise _failure("two-worker gate selection differs")
    for gate_observation, source in zip(
        gate_observations,
        candidates,
        strict=True,
    ):
        require_keys(
            gate_observation,
            {"format", "id", "run", "execution_sha256"},
            "native.inventory.two_worker_gate.observation",
        )
        run_two = next(
            (
                observation
                for observation in object_list(
                    source,
                    "observations",
                    "native.inventory.source.observations",
                )
                if integer_value(observation, "run") == 2
            ),
            None,
        )
        if (
            run_two is None
            or string_value(gate_observation, "format")
            != string_value(source, "format")
            or string_value(gate_observation, "id") != string_value(source, "id")
            or integer_value(gate_observation, "run") != 2
            or sha256_value(gate_observation, "execution_sha256")
            != sha256_value(object_value(run_two, "execution"), "sha256")
        ):
            raise _failure("two-worker gate observation differs")


def validate_convertibility_preflight(
    manifest: dict[str, JsonValue],
    sources: list[dict[str, JsonValue]],
) -> None:
    preflight = object_value(manifest, "convertibility_preflight")
    require_keys(
        preflight,
        {"status", "worker_count", "observation_run", "source_count"},
        "native.inventory.convertibility_preflight",
    )
    if (
        string_value(preflight, "status") != "PASSED"
        or integer_value(preflight, "worker_count") != 1
        or integer_value(preflight, "observation_run") != 1
        or integer_value(preflight, "source_count") != len(sources)
        or any(
            not any(
                integer_value(observation, "run") == 1
                for observation in object_list(
                    source,
                    "observations",
                    "native.inventory.source.observations",
                )
            )
            for source in sources
        )
    ):
        raise _failure("convertibility preflight evidence differs")


def _failure(detail: str) -> NativeUnitError:
    return NativeUnitError(NativeUnitFailure.OUTPUT_INVALID, None, None, detail)
