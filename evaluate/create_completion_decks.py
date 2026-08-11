#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# ///
# How to run: python3 evaluate/create_completion_decks.py --output-dir <dir>

from __future__ import annotations

import argparse
from contextlib import suppress
import json
import logging
from pathlib import Path
import shutil
import tempfile
from typing import Final

if __package__:
    from .completion_deck_features import FEATURES
    from .completion_deck_manifest import (
        ContractError,
        adjustment_cases,
        load_adjustments,
    )
    from .completion_deck_package import deck_bytes
    from .completion_deck_specs import build_decks
else:
    from completion_deck_features import FEATURES
    from completion_deck_manifest import (
        ContractError,
        adjustment_cases,
        load_adjustments,
    )
    from completion_deck_package import deck_bytes
    from completion_deck_specs import build_decks


DEFAULT_ADJUSTMENTS: Final = Path(__file__).with_name("preset_adjustments.json")
logger = logging.getLogger(__name__)


def generate(output: Path, adjustment_manifest: Path) -> None:
    _validate_output(output)
    official_names, rows = load_adjustments(adjustment_manifest)
    cases, adjustment_shapes = adjustment_cases(rows)
    decks = build_decks(adjustment_shapes)
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
                "stimulus": {
                    "part": feature.part,
                    "token": feature.token,
                    **(
                        {
                            "negative": {
                                "kind": feature.negative.kind,
                                "part": feature.negative.part,
                                "token": feature.negative.token,
                            }
                        }
                        if feature.negative
                        else {}
                    ),
                },
                "powerpoint_capture_required": True,
                "native_evidence": {"images": [], "metadata": None},
            }
            for feature in FEATURES
        ],
    }
    artifacts = {f"{deck.name}.pptx": deck_bytes(deck) for deck in decks}
    artifacts["manifest.json"] = (
        json.dumps(manifest, indent=2, sort_keys=True) + "\n"
    ).encode()
    _publish(output, artifacts)


def _publish(output: Path, artifacts: dict[str, bytes]) -> None:
    staging: Path | None = None
    primary: OSError | ContractError | None = None
    primary_message = ""
    try:
        output.parent.mkdir(parents=True, exist_ok=True)
        staging = Path(
            tempfile.mkdtemp(prefix=f".{output.name}.stage-", dir=output.parent)
        )
        for name, payload in sorted(artifacts.items()):
            if Path(name).name != name:
                raise OSError("artifact name must not contain a path")
            (staging / name).write_bytes(payload)
        _validate_output(output)
        existed = output.exists()
        if existed:
            output.rmdir()
        try:
            staging.replace(output)
            staging = None
        except OSError:
            if existed:
                with suppress(OSError):
                    output.mkdir()
            raise
    except ContractError as error:
        primary = error
        primary_message = str(error)
    except OSError as error:
        primary = error
        primary_message = f"OUTPUT_WRITE_ERROR path={output}"

    cleanup_error: OSError | None = None
    if staging is not None:
        try:
            shutil.rmtree(staging)
        except OSError as error:
            cleanup_error = error
    if cleanup_error is not None:
        cleanup_path = staging.absolute() if staging is not None else output.absolute()
        primary_message += (
            f" OUTPUT_CLEANUP_ERROR staging={cleanup_path} detail={cleanup_error}"
        )
    if primary is not None:
        raise ContractError(primary_message) from primary


def _validate_output(output: Path) -> None:
    try:
        if output.is_symlink():
            raise ContractError(f"OUTPUT_DIR_SYMLINK path={output}")
        if output.exists() and not output.is_dir():
            raise ContractError(f"OUTPUT_DIR_NOT_DIRECTORY path={output}")
        if output.is_dir() and any(output.iterdir()):
            raise ContractError(f"OUTPUT_DIR_NOT_EMPTY path={output}")
    except OSError as error:
        raise ContractError(f"OUTPUT_DIR_ERROR path={output}") from error


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
