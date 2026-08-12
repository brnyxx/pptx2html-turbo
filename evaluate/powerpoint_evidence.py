from __future__ import annotations

import argparse
import json
from pathlib import Path


EXACT_PROMOTION_FAMILIES = {
    "text-layout": [
        "basic_text_08_narrow_box_autofit",
        "basic_text_09_mixed_font_paragraph",
        "basic_text_10_bodypr_fidelity",
        "basic_text_11_wrap_gate_sentence",
        "basic_text_12_wrap_gate_unbreakable",
        "basic_text_13_autofit_modes",
        "basic_text_14_complex_script_fonts",
        "basic_text_15_mixed_script_single_run",
        "basic_text_16_cjk_autofit_wrap_gate",
        "basic_text_17_indic_complex_script_fonts",
        "basic_text_18_emoji_cluster_segments",
    ]
}

try:
    from evaluate.scaffold_powerpoint_golden_batch import (
        scaffold_powerpoint_golden_batch,
    )
    from evaluate.summarize_powerpoint_golden import summarize_powerpoint_golden_batch
    from evaluate.validate_powerpoint_golden import validate_powerpoint_golden_batch
except ModuleNotFoundError:
    from scaffold_powerpoint_golden_batch import scaffold_powerpoint_golden_batch
    from summarize_powerpoint_golden import summarize_powerpoint_golden_batch
    from validate_powerpoint_golden import validate_powerpoint_golden_batch


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)

    def add_common_paths(subparser: argparse.ArgumentParser) -> None:
        subparser.add_argument("--golden-set-dir", required=True)
        subparser.add_argument("--output-dir", required=True)
        subparser.add_argument("--output-json", type=Path)

    def emit(payload: dict[str, object], output_json: Path | None) -> None:
        text = json.dumps(payload, indent=2, ensure_ascii=False)
        print(text)
        if output_json is not None:
            output_json.write_text(text + "\n", encoding="utf-8")

    summary_parser = subparsers.add_parser("summary")
    add_common_paths(summary_parser)

    validate_parser = subparsers.add_parser("validate")
    add_common_paths(validate_parser)

    ready_parser = subparsers.add_parser("ready")
    add_common_paths(ready_parser)

    gate_parser = subparsers.add_parser("gate")
    add_common_paths(gate_parser)
    gate_parser.add_argument(
        "--family", required=True, choices=sorted(EXACT_PROMOTION_FAMILIES)
    )

    scaffold_parser = subparsers.add_parser("scaffold")
    add_common_paths(scaffold_parser)
    scaffold_parser.add_argument("--powerpoint-version", required=True)
    scaffold_parser.add_argument("--powerpoint-build", required=True)
    scaffold_parser.add_argument("--powerpoint-channel", required=True)
    scaffold_parser.add_argument("--windows-version", required=True)
    scaffold_parser.add_argument("--export-command", required=True)
    scaffold_parser.add_argument("--output-resolution", required=True)
    scaffold_parser.add_argument("--golden-set-revision", required=True)
    scaffold_parser.add_argument("--capture-timestamp", required=True)
    scaffold_parser.add_argument("--batch-id", required=True)

    args = parser.parse_args(argv)

    if args.command == "scaffold":
        summary = scaffold_powerpoint_golden_batch(
            args.golden_set_dir,
            args.output_dir,
            metadata={
                "producer": "Microsoft PowerPoint",
                "platform": "Windows",
                "powerpoint_version": args.powerpoint_version,
                "powerpoint_build": args.powerpoint_build,
                "powerpoint_channel": args.powerpoint_channel,
                "windows_version": args.windows_version,
                "export_command": args.export_command,
                "output_resolution": args.output_resolution,
                "golden_set_revision": args.golden_set_revision,
                "capture_timestamp": args.capture_timestamp,
                "batch_id": args.batch_id,
            },
        )
        emit(summary, args.output_json)
        return 0

    if args.command == "validate":
        summary = validate_powerpoint_golden_batch(args.golden_set_dir, args.output_dir)
        emit(summary, args.output_json)
        return 0

    summary = summarize_powerpoint_golden_batch(args.golden_set_dir, args.output_dir)
    if args.command == "gate":
        required_decks = EXACT_PROMOTION_FAMILIES[args.family]
        available_decks = {detail["name"] for detail in summary["deck_details"]}
        missing_required_decks = [
            deck for deck in required_decks if deck not in available_decks
        ]
        invalid_required_decks = [
            deck for deck in summary["invalid_metadata"] if deck in required_decks
        ]
        incomplete_required_decks = [
            detail["name"]
            for detail in summary["deck_details"]
            if detail["name"] in required_decks
            and (
                not detail["has_output"]
                or not detail["has_metadata"]
                or detail["name"] in invalid_required_decks
                or detail["name"] in summary["incomplete_slide_exports"]
            )
        ]
        payload = {
            "family": args.family,
            "required_decks": required_decks,
            "missing_required_decks": missing_required_decks,
            "invalid_required_decks": invalid_required_decks,
            "incomplete_required_decks": incomplete_required_decks,
            "batch_identity": summary["batch_identity"],
            "family_ready_for_exact_promotion": (
                not missing_required_decks
                and not invalid_required_decks
                and not incomplete_required_decks
                and summary["manifest_present"]
                and summary["manifest_deck_count_matches"]
                and summary["manifest_slide_count_matches"]
            ),
        }
        emit(payload, args.output_json)
        return 0 if payload["family_ready_for_exact_promotion"] else 1

    emit(summary, args.output_json)
    if args.command == "ready":
        return 0 if summary["evidence_ready_for_exact_promotion"] else 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
