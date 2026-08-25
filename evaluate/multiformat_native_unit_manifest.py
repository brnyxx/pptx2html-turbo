from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

from evaluate.multiformat_corpus_types import CorpusError
from evaluate.multiformat_native_unit_types import (
    NativeObservation,
    NativeUnitError,
    NativeUnitFailure,
    NativeUnitSource,
)
from evaluate.multiformat_schema import (
    JsonValue,
    object_value,
    sha256_file,
    sha256_value,
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
) -> dict[str, JsonValue]:
    executions = tuple(
        read_strict_object(observation.execution_path) for observation in observations
    )
    libreoffice, pdfinfo = _tool_identities(executions)
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
    return {
        "schema_version": 1,
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
        "sources": sources,
    }


def _tool_identities(
    executions: tuple[dict[str, JsonValue], ...],
) -> tuple[dict[str, JsonValue], dict[str, JsonValue]]:
    office: dict[str, JsonValue] | None = None
    pdfinfo: dict[str, JsonValue] | None = None
    for execution in executions:
        tools = object_value(execution, "tools")
        current_pdfinfo = object_value(tools, "pdfinfo")
        if pdfinfo is None:
            pdfinfo = current_pdfinfo
        elif current_pdfinfo != pdfinfo:
            raise _failure("pdfinfo identity differs across observations")
        if "libreoffice" in tools:
            current_office = object_value(tools, "libreoffice")
            if office is None:
                office = current_office
            elif current_office != office:
                raise _failure("LibreOffice identity differs across observations")
    if office is None or pdfinfo is None:
        raise _failure("captured tool identity is missing")
    return office, pdfinfo


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
