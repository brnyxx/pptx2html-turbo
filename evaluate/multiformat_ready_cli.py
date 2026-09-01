from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import TextIO

from evaluate.multiformat_ready_assembly_types import (
    ReadyAssemblyError,
    ReadyAssemblySummary,
)
from evaluate.multiformat_ready_types import ReadyInputPaths


class ReadySourceArguments(argparse.Namespace):
    def __init__(self) -> None:
        super().__init__()
        self.contract = Path()
        self.plan = Path()
        self.pptx_manifest = Path()
        self.docx_manifest = Path()
        self.xlsx_manifest = Path()
        self.pdf_manifest = Path()
        self.legacy_manifest = Path()
        self.public_config = Path()
        self.public_pool_manifest = Path()
        self.legacy_binary_config = Path()
        self.legacy_binary_manifest = Path()
        self.security_manifest = Path()
        self.routing = Path()
        self.font_bundle = Path()
        self.soffice = Path()
        self.pdfinfo = Path()
        self.native_inventory_root = Path()


def add_ready_source_arguments(parser: argparse.ArgumentParser) -> None:
    for option in (
        "--contract",
        "--plan",
        "--pptx-manifest",
        "--docx-manifest",
        "--xlsx-manifest",
        "--pdf-manifest",
        "--legacy-manifest",
        "--public-config",
        "--public-pool-manifest",
        "--legacy-binary-config",
        "--legacy-binary-manifest",
        "--security-manifest",
        "--routing",
        "--font-bundle",
        "--soffice",
        "--pdfinfo",
        "--native-inventory-root",
    ):
        _ = parser.add_argument(option, type=Path, required=True)


def ready_input_paths(arguments: ReadySourceArguments) -> ReadyInputPaths:
    return ReadyInputPaths(
        arguments.contract,
        arguments.plan,
        arguments.pptx_manifest,
        arguments.docx_manifest,
        arguments.xlsx_manifest,
        arguments.pdf_manifest,
        arguments.legacy_manifest,
        arguments.public_config,
        arguments.public_pool_manifest,
        arguments.legacy_binary_config,
        arguments.legacy_binary_manifest,
        arguments.security_manifest,
        arguments.routing,
        arguments.font_bundle,
        arguments.soffice,
        arguments.pdfinfo,
        arguments.native_inventory_root,
    )


def emit_summary(stream: TextIO, summary: ReadyAssemblySummary) -> None:
    _emit(stream, summary.to_json_value())


def emit_error(stream: TextIO, error: ReadyAssemblyError) -> None:
    _emit(stream, {"error": error.failure.value, "message": error.detail})


def _emit(stream: TextIO, value: dict[str, int | str]) -> None:
    _ = stream.write(
        json.dumps(value, ensure_ascii=True, separators=(",", ":"), sort_keys=True)
        + "\n"
    )
