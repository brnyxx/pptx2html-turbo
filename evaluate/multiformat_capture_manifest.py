from __future__ import annotations

from pathlib import Path

from evaluate.multiformat_capture_provenance import validate_capture_provenance
from evaluate.multiformat_capture_types import (
    ArtifactIdentity,
    CaptureFile,
    CaptureManifest,
    CaptureUnit,
)
from evaluate.multiformat_corpus_items import object_list, require_keys
from evaluate.multiformat_corpus_types import CorpusError
from evaluate.multiformat_evidence import EvidencePathError, resolve_evidence_path
from evaluate.multiformat_metric_types import (
    CorpusMetricSpec,
    MetricError,
)
from evaluate.multiformat_schema import (
    JsonValue,
    integer_value,
    object_value,
    sha256_file,
    sha256_value,
    string_value,
)
from evaluate.multiformat_strict_json import StrictJsonError, read_strict_object


def validate_capture_manifest(
    path: Path,
    role: str,
    spec: CorpusMetricSpec,
    contract_sha256: str,
    corpus_sha256: str,
    evaluator_sha256: str,
    oracle_lock_sha256: str,
    project_revision: str,
    evidence_root: Path,
    oracle_lock_path: Path | None = None,
) -> CaptureManifest:
    try:
        values = read_strict_object(path)
        required_fields = {
            "schema_version",
            "status",
            "role",
            "format",
            "producer",
            "runtime_sha256",
            "runtime_identity",
            "contract_sha256",
            "corpus_manifest_sha256",
            "evaluator_manifest_sha256",
            "oracle_lock_sha256",
            "network_isolation",
            "rendering",
            "upstream_manifest",
            "units",
            "files",
        }
        if role == "candidate":
            required_fields |= {"determinism_manifest", "execution_receipt"}
        elif oracle_lock_path is not None:
            required_fields |= {
                "office_batch_manifest",
                "execution_receipt",
            }
        require_keys(values, required_fields, "capture.schema")
        if (
            integer_value(values, "schema_version") != 1
            or string_value(values, "status") != "READY"
            or string_value(values, "role") != role
            or string_value(values, "format") != spec.document_format.value
            or sha256_value(values, "contract_sha256") != contract_sha256
            or sha256_value(values, "corpus_manifest_sha256") != corpus_sha256
            or sha256_value(values, "evaluator_manifest_sha256") != evaluator_sha256
            or sha256_value(values, "oracle_lock_sha256") != oracle_lock_sha256
            or string_value(values, "network_isolation") != "disabled"
        ):
            raise MetricError("metrics.binding.capture", role)
        validate_capture_provenance(
            values,
            role,
            spec,
            contract_sha256,
            corpus_sha256,
            evaluator_sha256,
            oracle_lock_sha256,
            project_revision,
            evidence_root,
            oracle_lock_path,
        )
        units: dict[str, CaptureUnit] = {}
        artifact_paths: set[str] = set()
        expected_units = spec.capture_identities()
        for unit in object_list(values, "units", "capture.units"):
            require_keys(
                unit,
                {
                    "unit_id",
                    "source_id",
                    "source_sha256",
                    "ordinal",
                    "png",
                    "inventory",
                },
                "capture.unit",
            )
            unit_id = string_value(unit, "unit_id")
            identity = (
                string_value(unit, "source_id"),
                sha256_value(unit, "source_sha256"),
                integer_value(unit, "ordinal"),
            )
            if (
                unit_id in units
                or unit_id not in expected_units
                or identity != expected_units[unit_id]
            ):
                raise MetricError("metrics.binding.capture", f"{role} unit set")
            png = _artifact_identity(
                object_value(unit, "png"),
                evidence_root,
                artifact_paths,
            )
            inventory = _artifact_identity(
                object_value(unit, "inventory"),
                evidence_root,
                artifact_paths,
            )
            units[unit_id] = CaptureUnit(unit_id, *identity, png, inventory)
        if set(units) != set(spec.pair_ids()):
            raise MetricError("metrics.binding.capture", f"{role} unit set")
        files: dict[str, CaptureFile] = {}
        for file_record in object_list(values, "files", "capture.files"):
            require_keys(
                file_record,
                {"source_id", "source_sha256", "html"},
                "capture.file",
            )
            source_id = string_value(file_record, "source_id")
            if source_id in files:
                raise MetricError("metrics.binding.capture", f"{role} file set")
            html = _artifact_identity(
                object_value(file_record, "html"),
                evidence_root,
                artifact_paths,
            )
            files[source_id] = CaptureFile(
                source_id,
                sha256_value(file_record, "source_sha256"),
                html,
            )
        expected_files = _expected_files(spec) if role == "candidate" else {}
        actual_files = {
            source_id: item.source_sha256 for source_id, item in files.items()
        }
        if actual_files != expected_files:
            raise MetricError("metrics.binding.capture", f"{role} file set")
        determinism_path = (
            resolve_evidence_path(
                evidence_root,
                string_value(
                    object_value(values, "determinism_manifest"),
                    "path",
                ),
            )
            if role == "candidate"
            else None
        )
        return CaptureManifest(units, files, determinism_path)
    except MetricError:
        raise
    except (
        CorpusError,
        EvidencePathError,
        StrictJsonError,
        OSError,
        TypeError,
        ValueError,
    ) as error:
        raise MetricError("metrics.binding.capture", role) from error


def _artifact_identity(
    binding: dict[str, JsonValue],
    evidence_root: Path,
    artifact_paths: set[str],
) -> ArtifactIdentity:
    require_keys(binding, {"path", "sha256"}, "capture.artifact")
    relative_path = string_value(binding, "path")
    digest = sha256_value(binding, "sha256")
    path = resolve_evidence_path(evidence_root, relative_path)
    if relative_path in artifact_paths or sha256_file(path) != digest:
        raise MetricError("metrics.binding.capture", relative_path)
    artifact_paths.add(relative_path)
    return ArtifactIdentity(relative_path, digest)


def _expected_files(spec: CorpusMetricSpec) -> dict[str, str]:
    result = {unit.source_id: unit.source_sha256 for unit in spec.conformance.values()}
    result.update(
        {source_id: item.source_sha256 for source_id, item in spec.blind.items()}
    )
    return result
