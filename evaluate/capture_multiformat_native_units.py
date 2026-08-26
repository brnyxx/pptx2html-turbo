from __future__ import annotations

import argparse
import json
import sys
from collections.abc import Sequence
from dataclasses import dataclass
from pathlib import Path
from typing import TextIO, cast

from evaluate.multiformat_native_unit_capture import (
    NativeUnitCaptureInputs,
    capture_native_unit_inventory,
)
from evaluate.multiformat_native_unit_types import NativeUnitError
from evaluate.multiformat_native_unit_validation import (
    NativeUnitInventorySummary,
    NativeUnitValidationInputs,
    validate_native_unit_inventory,
)
from evaluate.multiformat_schema import JsonValue


def main(argv: Sequence[str] | None = None) -> int:
    arguments = _parser().parse_args(argv)
    command = cast(str, arguments.command)
    trusted = _trusted_paths(arguments)
    try:
        if command == "capture":
            summary = capture_native_unit_inventory(
                NativeUnitCaptureInputs(
                    trusted.contract,
                    trusted.public_config,
                    trusted.public_pool_manifest,
                    trusted.routing,
                    trusted.font_manifest,
                    trusted.libreoffice,
                    trusted.pdfinfo,
                    cast(Path, arguments.output_dir),
                    cast(int, arguments.workers),
                    cast(Path | None, arguments.cache_dir),
                )
            )
        else:
            summary = validate_native_unit_inventory(
                NativeUnitValidationInputs(
                    trusted.contract,
                    trusted.public_config,
                    trusted.public_pool_manifest,
                    trusted.routing,
                    trusted.font_manifest,
                    trusted.libreoffice,
                    trusted.pdfinfo,
                    cast(Path, arguments.inventory_root),
                )
            )
    except NativeUnitError as error:
        _emit(
            sys.stderr,
            {"error": error.failure.value, "message": str(error)},
        )
        return 1
    _emit(sys.stdout, _summary_value(summary))
    return 0


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="capture_multiformat_native_units")
    commands = parser.add_subparsers(dest="command", required=True)
    capture = commands.add_parser("capture")
    _add_trusted_arguments(capture)
    _ = capture.add_argument("--output-dir", type=Path, required=True)
    _ = capture.add_argument("--workers", type=int, required=True)
    _ = capture.add_argument("--cache-dir", type=Path)
    validate = commands.add_parser("validate")
    _add_trusted_arguments(validate)
    _ = validate.add_argument("--inventory-root", type=Path, required=True)
    return parser


def _add_trusted_arguments(parser: argparse.ArgumentParser) -> None:
    _ = parser.add_argument("--contract", type=Path, required=True)
    _ = parser.add_argument("--public-config", type=Path, required=True)
    _ = parser.add_argument("--blind-manifest", type=Path, required=True)
    _ = parser.add_argument("--routing", type=Path, required=True)
    _ = parser.add_argument("--font-bundle", type=Path, required=True)
    _ = parser.add_argument("--soffice", type=Path, required=True)
    _ = parser.add_argument("--pdfinfo", type=Path, required=True)


@dataclass(frozen=True, slots=True)
class _TrustedPaths:
    contract: Path
    public_config: Path
    public_pool_manifest: Path
    routing: Path
    font_manifest: Path
    libreoffice: Path
    pdfinfo: Path


def _trusted_paths(arguments: argparse.Namespace) -> _TrustedPaths:
    return _TrustedPaths(
        contract=cast(Path, arguments.contract),
        public_config=cast(Path, arguments.public_config),
        public_pool_manifest=cast(Path, arguments.blind_manifest),
        routing=cast(Path, arguments.routing),
        font_manifest=cast(Path, arguments.font_bundle),
        libreoffice=cast(Path, arguments.soffice),
        pdfinfo=cast(Path, arguments.pdfinfo),
    )


def _summary_value(summary: NativeUnitInventorySummary) -> dict[str, JsonValue]:
    return {
        "files": summary.files,
        "manifest_sha256": summary.manifest_sha256,
        "observations": summary.observations,
        "sources": summary.sources,
        "total_units": summary.total_units,
    }


def _emit(stream: TextIO, value: dict[str, JsonValue]) -> None:
    _ = stream.write(
        json.dumps(value, ensure_ascii=True, separators=(",", ":"), sort_keys=True)
        + "\n"
    )


if __name__ == "__main__":
    raise SystemExit(main())
