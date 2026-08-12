from __future__ import annotations

import argparse
import json
from pathlib import Path

from pptx import Presentation

try:
    from evaluate.powerpoint_provenance import (
        PRODUCER,
        PLATFORM,
        canonical_json_sha256,
        sha256_file,
        validate_png,
        validate_provenance,
    )
except ModuleNotFoundError:
    from powerpoint_provenance import (
        PRODUCER,
        PLATFORM,
        canonical_json_sha256,
        sha256_file,
        validate_png,
        validate_provenance,
    )


class ScaffoldError(RuntimeError):
    pass


def scaffold_powerpoint_golden_batch(
    golden_set_dir: Path,
    output_dir: Path,
    metadata: dict[str, str],
) -> dict[str, object]:
    golden_set_dir, output_dir = Path(golden_set_dir), Path(output_dir)
    if not golden_set_dir.is_dir() or not output_dir.is_dir():
        raise ScaffoldError("Golden set and PowerPoint output directories must exist")
    errors = validate_provenance(metadata)
    if errors:
        raise ScaffoldError(",".join(errors))

    decks: list[dict[str, object]] = []
    total_slide_count = 0
    for deck_path in sorted(golden_set_dir.glob("*.pptx")):
        deck_name = deck_path.stem
        deck_output = output_dir / deck_name
        if not deck_output.is_dir():
            raise ScaffoldError(f"Missing PowerPoint output directory for deck '{deck_name}'")
        slide_count = len(Presentation(deck_path).slides)
        images = []
        for slide in range(1, slide_count + 1):
            try:
                images.append(validate_png(deck_output / f"Slide{slide}.PNG"))
            except OSError as error:
                raise ScaffoldError(str(error)) from error
        source_sha256 = sha256_file(deck_path)
        deck_metadata: dict[str, object] = {
            **metadata,
            "deck_name": deck_name,
            "slide_count": slide_count,
            "source_file": deck_path.name,
            "source_sha256": source_sha256,
            "images": images,
        }
        metadata_bytes = (
            json.dumps(deck_metadata, indent=2, ensure_ascii=False, sort_keys=True) + "\n"
        ).encode()
        metadata_path = deck_output / "metadata.json"
        metadata_path.write_bytes(metadata_bytes)
        decks.append(
            {
                "name": deck_name,
                "slide_count": slide_count,
                "output_dir": deck_name,
                "source_file": deck_path.name,
                "source_sha256": source_sha256,
                "metadata_sha256": sha256_file(metadata_path),
                "images": images,
            }
        )
        total_slide_count += slide_count

    manifest: dict[str, object] = {
        **metadata,
        "deck_count": len(decks),
        "total_slide_count": total_slide_count,
        "decks": decks,
    }
    manifest["report_sha256"] = canonical_json_sha256(manifest)
    (output_dir / "manifest.json").write_text(
        json.dumps(manifest, indent=2, ensure_ascii=False, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    return {
        "deck_count": len(decks),
        "slide_image_count": total_slide_count,
        "validated_decks": [deck["name"] for deck in decks],
        "batch_id": metadata["batch_id"],
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--golden-set-dir", type=Path, default=Path("golden_set"))
    parser.add_argument("--output-dir", type=Path, default=Path("powerpoint_golden"))
    parser.add_argument("--powerpoint-version", required=True)
    parser.add_argument("--powerpoint-build", required=True)
    parser.add_argument("--powerpoint-channel", required=True)
    parser.add_argument("--windows-version", required=True)
    parser.add_argument("--export-command", required=True)
    parser.add_argument("--output-resolution", required=True)
    parser.add_argument("--golden-set-revision", required=True)
    parser.add_argument("--capture-timestamp", required=True)
    parser.add_argument("--batch-id", required=True)
    args = parser.parse_args()
    summary = scaffold_powerpoint_golden_batch(
        args.golden_set_dir,
        args.output_dir,
        metadata={
            "producer": PRODUCER,
            "platform": PLATFORM,
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
    print(f"Scaffolded {summary['deck_count']} deck(s) and {summary['slide_image_count']} slide(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
