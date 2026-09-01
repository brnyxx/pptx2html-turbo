from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

from evaluate.multiformat_corpus_items import object_list
from evaluate.multiformat_corpus_types import CorpusError
from evaluate.multiformat_native_unit_types import (
    NativeCaptureTools,
    NativeCaptureTool,
    NativeObservation,
    NativeUnitError,
    NativeUnitFailure,
    NativeUnitSource,
)
from evaluate.multiformat_native_unit_capture_gates import (
    NativeTwoWorkerGate,
    convertibility_preflight_value,
    derive_two_worker_gate,
    two_worker_gate_value,
)
from evaluate.multiformat_schema import (
    JsonValue,
    integer_value,
    object_value,
    sha256_file,
    sha256_value,
    string_list,
    string_value,
)
from evaluate.multiformat_strict_json import read_strict_object


@dataclass(frozen=True, slots=True)
class NativeManifestInputs:
    contract: Path
    public_config: Path
    public_pool_manifest: Path
    workers: int
    operating_system: str
    architecture: str
    font_manifest_sha256: str
    font_environment_sha256: str


def build_native_unit_manifest(
    inputs: NativeManifestInputs,
    observations: tuple[NativeObservation, ...],
    capture_tools: NativeCaptureTools | None = None,
    two_worker_gate: NativeTwoWorkerGate | None = None,
) -> dict[str, JsonValue]:
    executions = tuple(_execution_record(observation) for observation in observations)
    libreoffice, pdfinfo = _tool_identities(executions, observations)
    if capture_tools is None:
        _add_recorded_probe(libreoffice, executions, "libreoffice_version")
        _add_recorded_probe(pdfinfo, executions, "pdfinfo_version")
    else:
        _add_probe(libreoffice, capture_tools.libreoffice)
        _add_probe(pdfinfo, capture_tools.pdfinfo)
    sources: list[JsonValue] = []
    for offset in range(0, len(observations), 2):
        pair = observations[offset : offset + 2]
        if len(pair) != 2:
            source = pair[0].source if pair else None
            raise _failure("native observation pair is incomplete", source)
        first, second = pair
        if (
            first.source != second.source
            or first.unit_count <= 0
            or first.unit_count != second.unit_count
        ):
            raise _failure("two clean observations disagree", first.source)
        sources.append(
            {
                "id": first.source.source_id,
                "format": first.source.document_format.value,
                "path": first.source.relative_path,
                "sha256": _source_sha256(executions[offset], first.source),
                "unit_count": first.unit_count,
                "observations": [_observation_value(item) for item in pair],
            }
        )
    gate = two_worker_gate or derive_two_worker_gate(observations)
    return {
        "schema_version": 2,
        "status": "CAPTURED",
        "contract_sha256": sha256_file(inputs.contract),
        "public_pool": {
            "config_sha256": sha256_file(inputs.public_config),
            "manifest_sha256": sha256_file(inputs.public_pool_manifest),
        },
        "routing": {"sha256": string_value(executions[0], "routing_sha256")},
        "tools": {"libreoffice": libreoffice, "pdfinfo": pdfinfo},
        "font": {
            "manifest_sha256": inputs.font_manifest_sha256,
            "environment_sha256": inputs.font_environment_sha256,
        },
        "runtime": {
            "os": inputs.operating_system,
            "architecture": inputs.architecture,
            "locale": "en-US",
            "lang": "en_US.UTF-8",
            "lc_all": "en_US.UTF-8",
            "timezone": "UTC",
            "worker_count": inputs.workers,
            "environment_keys": {
                "office": [
                    "FONTCONFIG_FILE",
                    "HOME",
                    "LANG",
                    "LC_ALL",
                    "PATH",
                    "TMPDIR",
                    "TZ",
                ],
                "pdf": ["HOME", "LANG", "LC_ALL", "PATH", "TMPDIR", "TZ"],
            },
        },
        "convertibility_preflight": convertibility_preflight_value(observations),
        "two_worker_gate": two_worker_gate_value(gate),
        "sources": sources,
    }


def _add_probe(
    value: dict[str, JsonValue],
    tool: NativeCaptureTool,
) -> None:
    if (
        sha256_value(value, "sha256") != tool.identity.sha256
        or string_value(value, "version") != tool.version
    ):
        raise _failure("captured tool probe identity differs")
    arguments: list[JsonValue] = list(tool.probe.arguments)
    probe: dict[str, JsonValue] = {
        "role": tool.probe.role,
        "arguments": arguments,
        "timeout_seconds": tool.probe.timeout_seconds,
        "exit_code": tool.probe.exit_code,
    }
    value["version_probe"] = probe


def _add_recorded_probe(
    value: dict[str, JsonValue],
    executions: tuple[dict[str, JsonValue], ...],
    role: str,
) -> None:
    for execution in executions:
        for process in object_list(
            execution,
            "processes",
            "native.execution.processes",
        ):
            if string_value(process, "role") != role:
                continue
            arguments: list[JsonValue] = list(string_list(process, "arguments"))
            probe: dict[str, JsonValue] = {
                "role": role,
                "arguments": arguments,
                "timeout_seconds": integer_value(process, "timeout_seconds"),
                "exit_code": integer_value(process, "exit_code"),
            }
            value["version_probe"] = probe
            return
    raise _failure("captured tool version probe is missing")


def _tool_identities(
    executions: tuple[dict[str, JsonValue], ...],
    observations: tuple[NativeObservation, ...],
) -> tuple[dict[str, JsonValue], dict[str, JsonValue]]:
    office: dict[str, JsonValue] | None = None
    pdfinfo: dict[str, JsonValue] | None = None
    for execution, observation in zip(executions, observations, strict=True):
        try:
            tools = object_value(execution, "tools")
            current_pdfinfo = object_value(tools, "pdfinfo")
            current_office = (
                object_value(tools, "libreoffice") if "libreoffice" in tools else None
            )
        except (CorpusError, TypeError, ValueError) as error:
            raise _failure(
                "captured tool identity is invalid",
                observation.source,
            ) from error
        if pdfinfo is None:
            pdfinfo = current_pdfinfo
        elif current_pdfinfo != pdfinfo:
            raise _failure(
                "pdfinfo identity differs across observations",
                observation.source,
            )
        if current_office is not None:
            if office is None:
                office = current_office
            elif current_office != office:
                raise _failure(
                    "LibreOffice identity differs across observations",
                    observation.source,
                )
    if office is None or pdfinfo is None:
        source = observations[0].source if observations else None
        raise _failure("captured tool identity is missing", source)
    return office, pdfinfo


def _execution_record(
    observation: NativeObservation,
) -> dict[str, JsonValue]:
    try:
        return read_strict_object(observation.execution_path)
    except (OSError, TypeError, ValueError) as error:
        raise _failure(
            "execution record is invalid",
            observation.source,
        ) from error


def _observation_value(
    observation: NativeObservation,
) -> dict[str, JsonValue]:
    base = (
        f"observations/{observation.source.document_format.value}/"
        f"{observation.source.source_id}/run-{observation.run}"
    )
    return {
        "run": observation.run,
        "workspace_nonce": observation.workspace_nonce,
        "path": base,
        "execution": {
            "path": f"{base}/execution.json",
            "sha256": observation.execution_sha256,
        },
        "reference_pdf": {
            "path": f"{base}/reference.pdf",
            "sha256": observation.reference_pdf_sha256,
        },
        "pdfinfo": {
            "path": f"{base}/pdfinfo.txt",
            "sha256": observation.pdfinfo_sha256,
        },
    }


def _source_sha256(
    execution: dict[str, JsonValue],
    source: NativeUnitSource,
) -> str:
    try:
        return sha256_value(object_value(execution, "source"), "sha256")
    except (CorpusError, TypeError, ValueError) as error:
        raise _failure("execution source identity is invalid", source) from error


def _failure(
    detail: str,
    source: NativeUnitSource | None = None,
) -> NativeUnitError:
    return NativeUnitError(
        NativeUnitFailure.OUTPUT_INVALID,
        source.document_format if source is not None else None,
        source.source_id if source is not None else None,
        detail,
    )
