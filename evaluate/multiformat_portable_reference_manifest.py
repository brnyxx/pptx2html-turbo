from __future__ import annotations

import platform
from collections.abc import Callable
from pathlib import Path
from typing import cast

from evaluate.multiformat_candidate_artifacts import (
    evidence_binding,
    write_canonical_json,
)
from evaluate.multiformat_candidate_sources import CandidateSourceSet
from evaluate.multiformat_capture_manifest import validate_capture_manifest
from evaluate.multiformat_inventory import parse_inventory
from evaluate.multiformat_metric_links import load_metric_spec
from evaluate.multiformat_office_oracle_batch import OfficeOracleBatchFile
from evaluate.multiformat_office_oracle_inventory import write_office_oracle_inventories
from evaluate.multiformat_portable_receipt import (
    PortableReceiptVerification,
    verify_portable_receipt,
)
from evaluate.multiformat_portable_receipt_trust import PortableReceiptTrustContext
from evaluate.multiformat_portable_reference_artifacts import artifact_records
from evaluate.multiformat_schema import JsonValue, sha256_file
from evaluate.multiformat_strict_json import read_strict_object


class PortableReferenceManifestError(ValueError):
    pass


ReceiptExecutor = Callable[[Path, Path, Path], None]


def write_portable_reference_manifests(
    root: Path,
    source_set: CandidateSourceSet,
    batches: list[OfficeOracleBatchFile],
    trust: PortableReceiptTrustContext,
    receipt_executor: Path,
    *,
    batch_id: str,
    execute: ReceiptExecutor,
) -> Path:
    units: list[dict[str, JsonValue]] = []
    signed: list[tuple[Path, str]] = []
    by_id = {batch.source_id: batch for batch in batches}
    for source in source_set.sources:
        batch = by_id.get(source.source_id)
        if batch is None or batch.source_sha256 != source.source_sha256:
            raise PortableReferenceManifestError("portable batch source set differs")
        inventories = write_office_oracle_inventories(
            batch,
            [unit.unit_id for unit in source.units],
            root / "inventories" / source.source_id,
        )
        signed.extend((path, "capture-unit-inventory") for path in inventories)
        signed.extend((unit.png, "capture-unit-png") for unit in batch.units)
        signed.extend(
            [
                (batch.pdf, "reference-pdf"),
                (batch.layout, "text-layout"),
                (batch.semantic, "semantic"),
            ]
        )
        for spec, unit, inventory in zip(
            source.units, batch.units, inventories, strict=True
        ):
            parsed = parse_inventory(inventory, spec.unit_id)
            # A portable reference with unattributable cells cannot back a
            # content claim, so it must not reach the signed manifest.
            if parsed.unattributed_cells:
                raise PortableReferenceManifestError(
                    "portable reference inventory has unattributed cells"
                )
            units.append(
                {
                    "unit_id": spec.unit_id,
                    "source_id": source.source_id,
                    "source_sha256": source.source_sha256,
                    "ordinal": spec.ordinal,
                    "png": evidence_binding(trust.evidence_root, unit.png),
                    "inventory": evidence_binding(trust.evidence_root, inventory),
                }
            )
    units.sort(key=lambda item: str(item["unit_id"]))
    runtime = root / "runtime.json"
    write_canonical_json(
        runtime,
        {
            "schema_version": 1,
            "role": "oracle",
            "producer": "libreoffice-poppler",
            "project_revision": trust.project_revision,
            "os": platform.system(),
            "architecture": platform.machine(),
            "python": platform.python_version(),
            "tools": {
                tool.role: {"version": tool.version, "sha256": tool.sha256}
                for tool in trust.tools
            },
            "artifacts": {},
        },
    )
    execution = root / "execution.json"
    write_canonical_json(
        execution,
        {
            "schema_version": 1,
            "status": "PASS",
            "role": "oracle",
            "project_revision": trust.project_revision,
            "evaluator_manifest_sha256": trust.evaluator_sha256,
            "corpus_manifest_sha256": trust.corpus_sha256,
            "network_isolation": "disabled",
            "source_count": len(source_set.sources),
            "unit_count": len(units),
            "external_requests": [],
            "determinism_runs": 1,
        },
    )
    signed.extend(
        [
            (runtime, "capture-runtime-identity"),
            (execution, "capture-execution-log"),
        ]
    )
    receipt = root / "portable-receipt.json"
    request = root / "portable-receipt-request.json"
    records = artifact_records(trust.evidence_root, signed)
    write_canonical_json(
        request,
        {
            "schema_version": 2,
            "scope_sha256": trust.scope_sha256,
            "batch_id": batch_id,
            "artifacts": cast(JsonValue, records),
        },
    )
    execute(receipt_executor, request, receipt)
    verified = verify_portable_receipt(receipt, PortableReceiptVerification(trust))
    if verified.scope_sha256 != trust.scope_sha256:
        raise PortableReferenceManifestError("portable receipt identity differs")
    common: dict[str, JsonValue] = {
        "schema_version": 1,
        "status": "READY",
        "role": "oracle",
        "format": source_set.document_format.value,
        "producer": "libreoffice-poppler",
        "runtime_sha256": sha256_file(runtime),
        "runtime_identity": evidence_binding(trust.evidence_root, runtime),
        "project_revision": trust.project_revision,
        "contract_sha256": trust.contract_sha256,
        "corpus_manifest_sha256": trust.corpus_sha256,
        "evaluator_manifest_sha256": trust.evaluator_sha256,
        "oracle_lock_sha256": trust.lock_sha256,
        "execution_receipt": evidence_binding(trust.evidence_root, receipt),
        "units": cast(JsonValue, units),
        "files": [],
    }
    upstream = root / "upstream.json"
    write_canonical_json(
        upstream,
        {**common, "execution_log": evidence_binding(trust.evidence_root, execution)},
    )
    capture = root / "capture.json"
    rendering = cast(
        JsonValue,
        {"dpi": None, "width": 960, "height": 540}
        if source_set.document_format.value in {"ppt", "pptx"}
        else {"dpi": 144, "width": None, "height": None},
    )
    outer = {key: value for key, value in common.items() if key != "project_revision"}
    write_canonical_json(
        capture,
        cast(
            JsonValue,
            {
                **outer,
                "network_isolation": "disabled",
                "rendering": rendering,
                "upstream_manifest": evidence_binding(trust.evidence_root, upstream),
            },
        ),
    )
    validate_portable_publication(capture, source_set, trust.evidence_root)
    by_role = {
        item.role: trust.evidence_root / item.path for item in trust.lock_artifacts
    }
    validate_capture_manifest(
        capture,
        "oracle",
        load_metric_spec(by_role["corpus-manifest"]),
        trust.contract_sha256,
        trust.corpus_sha256,
        trust.evaluator_sha256,
        trust.lock_sha256,
        trust.project_revision,
        trust.evidence_root,
        by_role["portable-lock"],
    )
    return capture


def validate_portable_publication(
    capture: Path, sources: CandidateSourceSet, root: Path
) -> None:
    """Narrow pre-publication validation until profile-aware gate integration lands."""
    value = read_strict_object(capture)
    if value.get("status") != "READY" or value.get("producer") != "libreoffice-poppler":
        raise PortableReferenceManifestError("portable capture identity differs")
    units = value.get("units")
    expected = {unit.unit_id for source in sources.sources for unit in source.units}
    if (
        not isinstance(units, list)
        or {
            unit_id
            for item in units
            if isinstance(item, dict)
            and isinstance((unit_id := item.get("unit_id")), str)
        }
        != expected
    ):
        raise PortableReferenceManifestError("portable capture unit set differs")
    for item in units:
        if not isinstance(item, dict):
            raise PortableReferenceManifestError("portable capture unit is invalid")
        for field in ("png", "inventory"):
            binding = item.get(field)
            if not isinstance(binding, dict):
                raise PortableReferenceManifestError(
                    "portable artifact binding is invalid"
                )
            path = root / str(binding.get("path"))
            if not path.is_file() or sha256_file(path) != binding.get("sha256"):
                raise PortableReferenceManifestError(
                    "portable artifact binding differs"
                )
