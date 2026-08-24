from __future__ import annotations

import argparse
from collections.abc import Sequence
from pathlib import Path

from evaluate.multiformat_legacy_conformance import generate_legacy_pairs
from evaluate.multiformat_legacy_runtime import (
    LegacyExternalTools,
    build_legacy_runtime,
)
from evaluate.multiformat_legacy_types import (
    LegacyConformanceError,
    LegacyPairGeneration,
)


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Materialize paired DOC, XLS, and PPT conformance snapshots.",
    )
    parser.add_argument("--contract", type=Path, required=True)
    parser.add_argument("--plan", type=Path, required=True)
    parser.add_argument("--docx-manifest", type=Path, required=True)
    parser.add_argument("--xlsx-manifest", type=Path, required=True)
    parser.add_argument("--pptx-manifest", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--soffice", type=Path, required=True)
    parser.add_argument("--pdfinfo", type=Path, required=True)
    parser.add_argument("--font-bundle", type=Path, required=True)
    arguments = parser.parse_args(argv)
    try:
        runtime = build_legacy_runtime(
            LegacyExternalTools(
                arguments.soffice,
                arguments.pdfinfo,
                arguments.font_bundle,
            )
        )
        generate_legacy_pairs(
            LegacyPairGeneration(
                arguments.contract,
                arguments.plan,
                (
                    arguments.docx_manifest,
                    arguments.xlsx_manifest,
                    arguments.pptx_manifest,
                ),
                arguments.output_dir,
            ),
            runtime,
        )
    except LegacyConformanceError as error:
        parser.error(str(error))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
