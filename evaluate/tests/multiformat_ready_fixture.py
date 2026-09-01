from __future__ import annotations

import hashlib
import shutil
from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path
from unittest import mock

from evaluate.build_multiformat_conformance_plan import build_conformance_plan
from evaluate.generate_multiformat_docx_conformance import generate_docx_conformance
from evaluate.generate_multiformat_pdf_conformance import generate_pdf_conformance
from evaluate.generate_multiformat_pptx_conformance import generate_pptx_conformance
from evaluate.generate_multiformat_xlsx_conformance import generate_xlsx_conformance
from evaluate.multiformat_candidate_artifacts import write_canonical_json
from evaluate.multiformat_corpus_items import object_list
from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_legacy_conformance import generate_legacy_pairs
from evaluate.multiformat_legacy_types import (
    LegacyPairGeneration,
    LegacyPairJob,
    LegacyPairRuntime,
    LegacyToolIdentity,
)
from evaluate.multiformat_native_unit_capture import (
    NativeUnitCaptureInputs,
    capture_native_unit_inventory,
)
from evaluate.multiformat_public_pool_sources import public_source_url
from evaluate.multiformat_ready_types import ReadyInputPaths
from evaluate.multiformat_schema import (
    JsonValue,
    object_value,
    sha256_file,
    string_value,
)
from evaluate.multiformat_security_snapshot import generate_security_snapshot
from evaluate.multiformat_source_fixture import write_positive_source
from evaluate.multiformat_strict_json import read_strict_object
from evaluate.tests.multiformat_native_unit_fixture import (
    RecordingNativeRunner,
    make_native_inventory_fixture,
)

ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "evaluate/multiformat/contract.v1.json"


class ReadyFixtureError(Exception):
    pass


@dataclass(frozen=True, slots=True)
class ReadyInputFixture:
    paths: ReadyInputPaths


def make_ready_input_fixture(root: Path) -> ReadyInputFixture:
    (root / "native-inputs").mkdir()
    native = make_native_inventory_fixture(root / "native-inputs")
    contract = root / "contract.v1.json"
    shutil.copyfile(CONTRACT, contract)
    capture_native_unit_inventory(
        NativeUnitCaptureInputs(
            contract,
            native.public_config,
            native.public_pool_manifest,
            native.routing,
            native.font_manifest,
            native.soffice,
            native.pdfinfo,
            native.output,
            1,
        ),
        runner=RecordingNativeRunner(),
        nonce_factory=_nonces(),
    )
    plan = root / "conformance-plan.json"
    build_conformance_plan(contract, plan)
    with (
        mock.patch(
            "evaluate.generate_multiformat_pptx_conformance._file_binding",
            return_value=[],
        ),
        mock.patch(
            "evaluate.generate_multiformat_docx_conformance._source_set_hash",
            return_value="1" * 64,
        ),
        mock.patch(
            "evaluate.generate_multiformat_xlsx_conformance._source_bindings",
            return_value=[],
        ),
    ):
        pptx = generate_pptx_conformance(contract, plan, root / "pptx")
        docx = generate_docx_conformance(contract, plan, root / "docx")
        xlsx = generate_xlsx_conformance(contract, plan, root / "xlsx")
    pdf = _pdf_snapshot(root, contract, plan)
    legacy = generate_legacy_pairs(
        LegacyPairGeneration(contract, plan, (docx, xlsx, pptx), root / "legacy"),
        LegacyPairRuntime(_materialize_legacy, _tools()),
    )
    binary_config, binary_manifest = _binary_snapshot(
        root, native.public_config, native.public_pool_manifest
    )
    security_root = root / "security"
    generate_security_snapshot(contract, security_root)
    paths = ReadyInputPaths(
        contract,
        plan,
        pptx,
        docx,
        xlsx,
        pdf,
        legacy,
        native.public_config,
        native.public_pool_manifest,
        binary_config,
        binary_manifest,
        security_root / "security-sources.json",
        native.routing,
        native.font_manifest,
        native.soffice,
        native.pdfinfo,
        native.output,
    )
    return ReadyInputFixture(paths)


def _pdf_snapshot(root: Path, contract: Path, plan: Path) -> Path:
    def converter(inputs: tuple[Path, ...], output: Path, profile: Path) -> None:
        profile.mkdir()
        for source in inputs:
            write_positive_source(output / f"{source.stem}.pdf", "pdf", source.stem)

    def canonicalizer(source: Path, destination: Path) -> None:
        shutil.copyfile(source, destination)

    tools: dict[str, JsonValue] = {
        "soffice_sha256": "1" * 64,
        "soffice_version": "LibreOffice fixture",
        "pdfinfo_sha256": "2" * 64,
        "pdfinfo_version": "pdfinfo fixture",
        "pdftocairo_sha256": "3" * 64,
        "pdftocairo_version": "pdftocairo fixture",
        "font_environment_sha256": "4" * 64,
    }
    return generate_pdf_conformance(
        contract,
        plan,
        root / "pdf",
        converter=converter,
        canonicalizer=canonicalizer,
        page_counter=lambda _: 1,
        tools=tools,
    )


def _materialize_legacy(job: LegacyPairJob) -> int:
    write_positive_source(job.destination, job.document_format.value, job.case_id)
    return 1


def _tools() -> LegacyToolIdentity:
    return LegacyToolIdentity(
        "1" * 64,
        "LibreOffice fixture",
        "2" * 64,
        "pdfinfo fixture",
        "3" * 64,
    )


def _binary_snapshot(
    root: Path, public_config: Path, blind_manifest: Path
) -> tuple[Path, Path]:
    public = read_strict_object(public_config)
    public_formats = object_value(public, "formats")
    config_formats: dict[str, JsonValue] = {}
    manifest_formats: dict[str, JsonValue] = {}
    output = root / "binary"
    for document_format in (DocumentFormat.DOC, DocumentFormat.XLS, DocumentFormat.PPT):
        public_value = public_formats[document_format.value]
        if not isinstance(public_value, dict):
            raise ReadyFixtureError("public fixture format is invalid")
        groups = object_list(public_value, "groups", "ready.fixture.groups")
        config_formats[document_format.value] = {
            "expected_count": 40,
            "groups": [
                {"producer": string_value(group, "producer"), "quota": 8}
                for group in groups
            ],
        }
        sources: list[JsonValue] = []
        for group in groups:
            producer = string_value(group, "producer")
            repository = string_value(group, "repository")
            commit = string_value(group, "commit")
            for ordinal in range(1, 9):
                source_id = f"binary-{document_format.value}-{producer}-{ordinal:03d}"
                relative = f"sources/{document_format.value}/{producer}/{ordinal:03d}.{document_format.value}"
                source = output / relative
                source.parent.mkdir(parents=True, exist_ok=True)
                write_positive_source(source, document_format.value, source_id)
                repository_path = f"binary/{document_format.value}/{producer}/{ordinal:03d}.{document_format.value}"
                sources.append(
                    {
                        "id": source_id,
                        "path": relative,
                        "sha256": sha256_file(source),
                        "producer": producer,
                        "source_uri": public_source_url(
                            repository, commit, repository_path
                        ),
                        "template_family": f"{producer}-binary",
                        "repository": repository,
                        "commit": commit,
                        "repository_path": repository_path,
                        "license_spdx": string_value(group, "license_spdx"),
                        "applicable_metrics": ["visual", "content", "layout"],
                        "background": "light",
                        "independently_authored": True,
                    }
                )
        manifest_formats[document_format.value] = {
            "expected_count": 40,
            "sources": sources,
        }
    config = root / "legacy-binary-config.json"
    write_canonical_json(
        config,
        {
            "schema_version": 1,
            "source_catalog_sha256": sha256_file(public_config),
            "formats": config_formats,
        },
    )
    manifest = output / "legacy-binary-pool.json"
    write_canonical_json(
        manifest,
        {
            "schema_version": 1,
            "status": "COLLECTED",
            "selection_config_sha256": sha256_file(config),
            "source_catalog_sha256": sha256_file(public_config),
            "blind_manifest_sha256": sha256_file(blind_manifest),
            "formats": manifest_formats,
        },
    )
    return config, manifest


def _nonces() -> Callable[[], str]:
    index = 0

    def nonce() -> str:
        nonlocal index
        index += 1
        return hashlib.sha256(f"ready:{index}".encode()).hexdigest()

    return nonce
