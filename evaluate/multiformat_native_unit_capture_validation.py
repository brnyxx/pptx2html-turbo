from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

from evaluate.multiformat_native_unit_stable_validation import StableFile, stable_file
from evaluate.multiformat_native_unit_types import NativeUnitError, NativeUnitFailure
from evaluate.multiformat_strict_json import MAX_JSON_BYTES


@dataclass(frozen=True, slots=True)
class NativeTrustedInputs:
    contract: Path
    public_config: Path
    public_pool_manifest: Path
    routing: Path
    font_manifest: Path


@dataclass(frozen=True, slots=True)
class NativeTrustedState:
    inputs: NativeTrustedInputs
    identities: tuple[StableFile, ...]


def snapshot_trusted_inputs(inputs: NativeTrustedInputs) -> NativeTrustedState:
    return NativeTrustedState(
        inputs,
        tuple(
            stable_file(
                path,
                executable=False,
                maximum=MAX_JSON_BYTES,
            )
            for path in _paths(inputs)
        ),
    )


def revalidate_trusted_inputs(state: NativeTrustedState) -> None:
    for path, expected in zip(
        _paths(state.inputs),
        state.identities,
        strict=True,
    ):
        current = stable_file(
            path,
            executable=False,
            maximum=MAX_JSON_BYTES,
        )
        if current != expected:
            raise NativeUnitError(
                NativeUnitFailure.OUTPUT_INVALID,
                None,
                None,
                f"trusted capture input changed: {path.name}",
            )


def _paths(inputs: NativeTrustedInputs) -> tuple[Path, ...]:
    return (
        inputs.contract,
        inputs.public_config,
        inputs.public_pool_manifest,
        inputs.routing,
        inputs.font_manifest,
    )
