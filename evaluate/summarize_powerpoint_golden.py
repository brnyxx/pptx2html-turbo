from __future__ import annotations

import argparse
import json
from pathlib import Path

from pptx import Presentation

try:
    from evaluate.powerpoint_provenance import validate_provenance
    from evaluate.validate_powerpoint_golden import (
        REQUIRED_METADATA_FIELDS,
        ValidationError,
        validate_powerpoint_golden_batch,
    )
except ModuleNotFoundError:
    from powerpoint_provenance import validate_provenance
    from validate_powerpoint_golden import (
        REQUIRED_METADATA_FIELDS,
        ValidationError,
        validate_powerpoint_golden_batch,
    )


def summarize_powerpoint_golden_batch(
    golden_set_dir: Path, output_dir: Path
) -> dict[str, object]:
    golden_set_dir = Path(golden_set_dir)
    output_dir = Path(output_dir)

    golden_decks = sorted(deck.stem for deck in golden_set_dir.glob("*.pptx"))
    missing_decks: list[str] = []
    missing_metadata: list[str] = []
    invalid_metadata: list[str] = []
    incomplete_slide_exports: list[str] = []
    deck_details: list[dict[str, object]] = []

    for deck_path in sorted(golden_set_dir.glob("*.pptx")):
        deck_name = deck_path.stem
        expected_slide_count = len(Presentation(deck_path).slides)
        deck_output = output_dir / deck_name

        if not deck_output.is_dir():
            missing_decks.append(deck_name)
            deck_details.append(
                {
                    "name": deck_name,
                    "expected_slide_count": expected_slide_count,
                    "has_output": False,
                    "has_metadata": False,
                    "captured_slide_count": 0,
                }
            )
            continue

        metadata_path = deck_output / "metadata.json"
        has_metadata = metadata_path.is_file()
        if not has_metadata:
            missing_metadata.append(deck_name)
            metadata = {}
        else:
            metadata = json.loads(metadata_path.read_text(encoding="utf-8"))
            missing_fields = [
                field for field in REQUIRED_METADATA_FIELDS if not metadata.get(field)
            ]
            if missing_fields or validate_provenance(metadata):
                invalid_metadata.append(deck_name)

        slide_pngs = sorted(path.name for path in deck_output.glob("Slide*.PNG"))
        expected_slide_names = [
            f"Slide{idx}.PNG" for idx in range(1, expected_slide_count + 1)
        ]
        captured_slide_count = len(slide_pngs)
        if slide_pngs != expected_slide_names:
            incomplete_slide_exports.append(deck_name)

        deck_details.append(
            {
                "name": deck_name,
                "expected_slide_count": expected_slide_count,
                "has_output": True,
                "has_metadata": has_metadata,
                "captured_slide_count": captured_slide_count,
            }
        )

    manifest_path = output_dir / "manifest.json"
    manifest_present = manifest_path.is_file()
    manifest_deck_names: list[str] = []
    manifest_deck_count_matches = False
    manifest_slide_count_matches = False
    batch_identity = {
        "batch_id": None,
        "producer": None,
        "platform": None,
        "golden_set_revision": None,
        "capture_timestamp": None,
        "powerpoint_version": None,
        "powerpoint_build": None,
        "powerpoint_channel": None,
        "windows_version": None,
        "output_resolution": None,
    }
    if manifest_present:
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        manifest_deck_names = [deck["name"] for deck in manifest.get("decks", [])]
        manifest_deck_count_matches = manifest.get("deck_count") == len(golden_decks)
        expected_total_slide_count = sum(
            detail["expected_slide_count"] for detail in deck_details
        )
        manifest_slide_count_matches = (
            manifest.get("total_slide_count") == expected_total_slide_count
        )
        for key in batch_identity:
            batch_identity[key] = manifest.get(key)

    provenance_errors: list[str] = []
    preliminarily_complete = (
        not missing_decks
        and not missing_metadata
        and not invalid_metadata
        and not incomplete_slide_exports
        and manifest_present
        and manifest_deck_count_matches
        and manifest_slide_count_matches
    )
    if preliminarily_complete:
        try:
            validate_powerpoint_golden_batch(golden_set_dir, output_dir)
        except ValidationError as error:
            provenance_errors.append(str(error))
    evidence_ready_for_exact_promotion = preliminarily_complete and not provenance_errors

    return {
        "golden_deck_count": len(golden_decks),
        "captured_deck_count": len(golden_decks) - len(missing_decks),
        "missing_decks": missing_decks,
        "missing_metadata": missing_metadata,
        "invalid_metadata": invalid_metadata,
        "incomplete_slide_exports": incomplete_slide_exports,
        "manifest_present": manifest_present,
        "manifest_deck_names": manifest_deck_names,
        "manifest_deck_count_matches": manifest_deck_count_matches,
        "manifest_slide_count_matches": manifest_slide_count_matches,
        "batch_identity": batch_identity,
        "provenance_errors": provenance_errors,
        "evidence_ready_for_exact_promotion": evidence_ready_for_exact_promotion,
        "deck_details": deck_details,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--golden-set-dir", type=Path, default=Path("golden_set"))
    parser.add_argument("--output-dir", type=Path, default=Path("powerpoint_golden"))
    args = parser.parse_args()

    summary = summarize_powerpoint_golden_batch(args.golden_set_dir, args.output_dir)
    print(json.dumps(summary, indent=2, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
