#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# ///
# How to run: python3 evaluate/create_completion_decks.py --output-dir <dir>

from __future__ import annotations

import argparse
import json
import logging
from pathlib import Path
from typing import Final

from completion_deck_manifest import (
    FEATURES,
    ContractError,
    adjustment_cases,
    load_adjustments,
)
from completion_deck_package import write_deck
from completion_deck_specs import build_decks


DEFAULT_ADJUSTMENTS: Final = Path(__file__).with_name("preset_adjustments.json")
logger = logging.getLogger(__name__)


def generate(output: Path, adjustment_manifest: Path) -> None:
    official_names, rows = load_adjustments(adjustment_manifest)
    cases, adjustment_shapes = adjustment_cases(rows)
    decks = build_decks(adjustment_shapes)
    output.mkdir(parents=True, exist_ok=True)
    for deck in decks:
        write_deck(output, deck)
    manifest = {
        "schema_version": 2,
        "powerpoint_capture_required": True,
        "native_evidence": {"images": [], "metadata": None},
        "adjustment_case_source": {
            "file": adjustment_manifest.name,
            "official_preset_count": len(official_names),
        },
        "adjustment_case_scaffold": cases,
        "decks": [{"name": deck.name, "file": f"{deck.name}.pptx"} for deck in decks],
        "features": [
            {
                "task": feature.task,
                "deck": f"{feature.deck}.pptx",
                "id": feature.feature_id,
                "stimulus": {"part": feature.part, "token": feature.token},
                "powerpoint_capture_required": True,
                "native_evidence": {"images": [], "metadata": None},
            }
            for feature in FEATURES
        ],
    }
    (output / "manifest.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Generate deterministic PPTX completion fixtures."
    )
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--adjustment-manifest", type=Path, default=DEFAULT_ADJUSTMENTS)
    args = parser.parse_args()
    try:
        generate(args.output_dir, args.adjustment_manifest)
    except ContractError as error:
        logger.error("%s", error)
        return 2
    return 0


if __name__ == "__main__":
    logging.basicConfig(format="%(levelname)s: %(message)s")
    raise SystemExit(main())
