from __future__ import annotations

import hashlib
import json
from dataclasses import dataclass
from pathlib import Path

from evaluate.multiformat_corpus_types import DocumentFormat
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
        )


class RecordingNativeRunner:
    """Deterministic fake for one bounded process request at a time."""

    def __init__(self) -> None:
        self.requests: list[NativeProcessRequest] = []
        self.exit_code: int = 0
        self.failure: str | None = None
        self.write_pdf: bool = True
        self.pdfinfo_output: bytes = b"Pages:           1\n"
        self.stdout_output: bytes = b""
        self.stderr_output: bytes = b""

    def __call__(self, request: NativeProcessRequest) -> int:
        self.requests.append(request)
        if self.failure is not None:
            from evaluate.multiformat_candidate_process import CandidateProcessError

            raise CandidateProcessError(self.failure)
        _ = request.stdout_path.write_bytes(self.stdout_output)
        _ = request.stderr_path.write_bytes(self.stderr_output)
        command = request.command
        if command[-1:] == ("--version",):
            _ = request.stdout_path.write_bytes(b"LibreOffice 26.2.2.2\n")
        elif command[-1:] == ("-v",):
            _ = request.stdout_path.write_bytes(b"pdfinfo version 26.03.0\n")
        elif "--convert-to" in command:
            if self.write_pdf:
                output_dir = Path(command[command.index("--outdir") + 1])
                source = Path(command[-1])
                _ = output_dir.mkdir(parents=True, exist_ok=True)
                _ = (output_dir / f"{source.stem}.pdf").write_bytes(
                    b"%PDF-1.4\nfixture-native-reference\n"
                )
        elif command[0].endswith("pdfinfo"):
            _ = request.stdout_path.write_bytes(self.pdfinfo_output)
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
