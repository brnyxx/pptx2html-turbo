from __future__ import annotations

import hashlib
import json
from dataclasses import dataclass
from pathlib import Path

from evaluate.multiformat_candidate_artifacts import write_canonical_json
from evaluate.multiformat_candidate_process import (
    CandidateProcessError,
    CandidateProcessFailure,
)
from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_font_snapshot import generate_font_snapshot
from evaluate.multiformat_native_unit_types import (
    NativeProcessRequest,
    NativeUnitRequest,
    NativeUnitRuntime,
    NativeUnitSource,
)
from evaluate.multiformat_reference_routing import (
    RoutingIdentity,
    load_reference_routing,
)
from evaluate.multiformat_schema import JsonValue, sha256_file
from evaluate.multiformat_source_fixture import write_positive_source

ROOT = Path(__file__).resolve().parents[2]
ROUTING_TABLE = ROOT / "evaluate/multiformat/reference-routing.v1.json"


@dataclass(frozen=True, slots=True)
class NativeUnitFixture:
    source: Path
    soffice: Path
    pdfinfo: Path
    font_bundle: Path
    routing: RoutingIdentity

    def request(
        self,
        root: Path,
        document_format: DocumentFormat,
        *,
        run: int = 1,
    ) -> NativeUnitRequest:
        return NativeUnitRequest(
            source=NativeUnitSource(
                source_id=f"blind-{document_format.value}-001",
                document_format=document_format,
                path=self.source,
                relative_path=f"sources/{self.source.name}",
            ),
            runtime=NativeUnitRuntime(
                soffice=self.soffice,
                pdfinfo=self.pdfinfo,
                font_bundle=self.font_bundle,
                routing=self.routing,
            ),
            observation_dir=root / f"observation-{document_format.value}-{run}",
            run=run,
            nonce=hashlib.sha256(f"{document_format.value}:{run}".encode()).hexdigest(),
        )


@dataclass(frozen=True, slots=True)
class NativeInventoryFixture:
    contract: Path
    public_config: Path
    public_pool_manifest: Path
    routing: Path
    font_manifest: Path
    soffice: Path
    pdfinfo: Path
    output: Path


class RecordingNativeRunner:
    """Deterministic fake for one bounded process request at a time."""

    def __init__(self) -> None:
        self.requests: list[NativeProcessRequest] = []
        self.exit_code: int = 0
        self.failure: CandidateProcessFailure | None = None
        self.write_pdf: bool = True
        self.pdfinfo_output: bytes = b"Pages:           1\n"
        self.stdout_output: bytes = b""
        self.stderr_output: bytes = b""
        self.office_version_output: bytes = b"LibreOffice 26.2.2.2\n"
        self.pdfinfo_version_output: bytes = b"pdfinfo version 26.03.0\n"
        self.pdf_output: bytes = b"%PDF-1.4\nfixture-native-reference\n"
        self.mutate_pdf: bool = False
        self.missing_stdout: bool = False
        self.missing_stderr: bool = False
        self.mutate_tool: Path | None = None

    def __call__(self, request: NativeProcessRequest) -> int:
        self.requests.append(request)
        if self.failure is not None:
            raise CandidateProcessError(self.failure)
        if not self.missing_stdout:
            _ = request.stdout_path.write_bytes(self.stdout_output)
        if not self.missing_stderr:
            _ = request.stderr_path.write_bytes(self.stderr_output)
        command = request.command
        if command[-1:] == ("--version",):
            if not self.missing_stdout:
                _ = request.stdout_path.write_bytes(self.office_version_output)
        elif command[-1:] == ("-v",):
            if not self.missing_stdout:
                _ = request.stdout_path.write_bytes(self.pdfinfo_version_output)
        elif "--convert-to" in command:
            if self.write_pdf:
                output_dir = Path(command[command.index("--outdir") + 1])
                source = Path(command[-1])
                _ = output_dir.mkdir(parents=True, exist_ok=True)
                _ = (output_dir / f"{source.stem}.pdf").write_bytes(self.pdf_output)
        elif command[0].endswith("pdfinfo"):
            if not self.missing_stdout:
                _ = request.stdout_path.write_bytes(self.pdfinfo_output)
            if self.mutate_pdf:
                _ = Path(command[-1]).write_bytes(b"%PDF-1.4\nmutated\n")
        if self.mutate_tool is not None and Path(command[0]) == self.mutate_tool:
            _ = self.mutate_tool.write_bytes(b"replacement-tool")
        return self.exit_code


def make_native_unit_fixture(root: Path) -> NativeUnitFixture:
    source = root / "source.docx"
    _ = source.write_bytes(b"fixture-source-bytes")
    soffice = root / "soffice"
    pdfinfo = root / "pdfinfo"
    _ = soffice.write_bytes(b"fixture-soffice")
    _ = pdfinfo.write_bytes(b"fixture-pdfinfo")
    _ = soffice.chmod(0o755)
    _ = pdfinfo.chmod(0o755)
    font = root / "fixture.ttf"
    _ = font.write_bytes(b"fixture-font")
    font_bundle = root / "font-bundle.json"
    _ = font_bundle.write_text(
        json.dumps(
            {
                "schema_version": 1,
                "fonts": [
                    {
                        "path": font.name,
                        "sha256": hashlib.sha256(font.read_bytes()).hexdigest(),
                    }
                ],
            }
        ),
        encoding="utf-8",
    )
    return NativeUnitFixture(
        source=source,
        soffice=soffice,
        pdfinfo=pdfinfo,
        font_bundle=font_bundle,
        routing=load_reference_routing(ROUTING_TABLE),
    )


def make_native_inventory_fixture(root: Path) -> NativeInventoryFixture:
    pool = root / "pool"
    pool.mkdir()
    config_formats: dict[str, JsonValue] = {}
    manifest_formats: dict[str, JsonValue] = {}
    for document_format in sorted(DocumentFormat, key=lambda item: item.value):
        groups: list[JsonValue] = []
        sources: list[JsonValue] = []
        for group_index in range(5):
            producer = f"{document_format.value}-producer-{group_index + 1}"
            repository = f"owner/{document_format.value}-repo-{group_index + 1}"
            commit = hashlib.sha256(repository.encode()).hexdigest()[:40]
            groups.append(
                {
                    "producer": producer,
                    "repository": repository,
                    "commit": commit,
                    "license_spdx": "MIT",
                    "quota": 15,
                    "path_prefixes": ["fixtures/"],
                    "static_paths": [],
                }
            )
            for item_index in range(15):
                ordinal = group_index * 15 + item_index + 1
                source_id = f"blind-{document_format.value}-{ordinal:03d}"
                relative = (
                    f"sources/{document_format.value}/{producer}/"
                    f"{ordinal:03d}.{document_format.value}"
                )
                source = pool / relative
                source.parent.mkdir(parents=True, exist_ok=True)
                write_positive_source(
                    source,
                    document_format.value,
                    source_id,
                )
                sources.append(
                    {
                        "id": source_id,
                        "path": relative,
                        "sha256": sha256_file(source),
                        "producer": producer,
                        "source_uri": f"https://example.invalid/{relative}",
                        "template_family": f"{producer}-template",
                        "repository": repository,
                        "commit": commit,
                        "repository_path": f"fixtures/{source.name}",
                        "license_spdx": "MIT",
                        "applicable_metrics": ["visual", "content", "layout"],
                        "background": "light",
                    }
                )
        config_formats[document_format.value] = {
            "expected_count": 75,
            "groups": groups,
        }
        manifest_formats[document_format.value] = {
            "expected_count": 75,
            "sources": sources,
        }
    config = root / "public-config.json"
    manifest = pool / "public-pool.json"
    write_canonical_json(config, {"schema_version": 1, "formats": config_formats})
    write_canonical_json(
        manifest,
        {
            "schema_version": 1,
            "status": "COLLECTED",
            "formats": manifest_formats,
        },
    )
    font_source = root / "font-source"
    font_source.mkdir()
    _ = (font_source / "fixture.ttf").write_bytes(b"fixture-font")
    font_snapshot = root / "font-snapshot"
    _ = generate_font_snapshot((font_source,), font_snapshot)
    soffice = root / "soffice"
    pdfinfo = root / "pdfinfo"
    _ = soffice.write_text(
        '#!/bin/sh\nif [ "${1-}" = "--version" ]; then\n  printf \'LibreOffice 26.2.2.2\\n\'\n  exit 0\nfi\noutdir=\'\'\nsource=\'\'\nprevious=\'\'\nfor argument in "$@"; do\n  if [ "$previous" = "--outdir" ]; then outdir="$argument"; fi\n  previous="$argument"\n  source="$argument"\ndone\nbase="${source##*/}"\nstem="${base%.*}"\nprintf \'%%PDF-1.4\\nfixture\\n\' > "$outdir/$stem.pdf"\n',
        encoding="utf-8",
    )
    _ = pdfinfo.write_text(
        "#!/bin/sh\nif [ \"${1-}\" = \"-v\" ]; then\n  printf 'pdfinfo version 26.03.0\\n' >&2\nelse\n  printf 'Pages:           1\\n'\nfi\n",
        encoding="utf-8",
    )
    _ = soffice.chmod(0o755)
    _ = pdfinfo.chmod(0o755)
    return NativeInventoryFixture(
        ROOT / "evaluate/multiformat/contract.v1.json",
        config,
        manifest,
        ROUTING_TABLE,
        font_snapshot / "font-bundle.json",
        soffice,
        pdfinfo,
        root / "native-units",
    )
