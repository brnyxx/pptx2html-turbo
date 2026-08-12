from __future__ import annotations

import argparse
from pathlib import Path

from pptx import Presentation

try:
    from evaluate.powerpoint_provenance import (
        REQUIRED_PROVENANCE_FIELDS,
        ProvenanceError,
        canonical_json_sha256,
        matching_provenance,
        read_json,
        sha256_file,
        validate_png,
        validate_provenance,
    )
except ModuleNotFoundError:
    from powerpoint_provenance import (
        REQUIRED_PROVENANCE_FIELDS,
        ProvenanceError,
        canonical_json_sha256,
        matching_provenance,
        read_json,
        sha256_file,
        validate_png,
        validate_provenance,
    )

REQUIRED_METADATA_FIELDS = REQUIRED_PROVENANCE_FIELDS


class ValidationError(RuntimeError):
    pass


def validate_powerpoint_golden_batch(
    golden_set_dir: Path, output_dir: Path
) -> dict[str, object]:
    golden_set_dir, output_dir = Path(golden_set_dir), Path(output_dir)
    if not golden_set_dir.is_dir() or not output_dir.is_dir():
        raise ValidationError("Golden set and PowerPoint output directories must exist")
    try:
        manifest_path = output_dir / "manifest.json"
        manifest = read_json(manifest_path)
        errors = validate_provenance(manifest)
        recorded_report = manifest.get("report_sha256")
        report_payload = dict(manifest)
        report_payload.pop("report_sha256", None)
        if recorded_report != canonical_json_sha256(report_payload):
            errors.append("REPORT_SHA256_MISMATCH")
        manifest_rows = manifest.get("decks")
        if not isinstance(manifest_rows, list) or len(manifest_rows) > 256:
            errors.append("MANIFEST_DECKS_INVALID")
            manifest_rows = []
        by_name = {
            row.get("name"): row
            for row in manifest_rows
            if isinstance(row, dict) and isinstance(row.get("name"), str)
        }
        validated: list[str] = []
        image_count = 0
        details: list[dict[str, object]] = []
        deck_paths = sorted(golden_set_dir.glob("*.pptx"))
        if len(deck_paths) > 256:
            errors.append("GOLDEN_DECK_COUNT_EXCEEDED")
        for deck_path in deck_paths:
            name = deck_path.stem
            row = by_name.get(name)
            metadata_path = output_dir / name / "metadata.json"
            if not isinstance(row, dict):
                errors.append(f"MANIFEST_DECK_MISSING:{name}")
                continue
            metadata = read_json(metadata_path)
            errors.extend(f"{name}:{error}" for error in validate_provenance(metadata))
            if not matching_provenance(manifest, metadata):
                errors.append(f"PROVENANCE_CROSSLINK_MISMATCH:{name}")
            source_hash = sha256_file(deck_path)
            if row.get("source_file") != deck_path.name or row.get("source_sha256") != source_hash or metadata.get("source_sha256") != source_hash:
                errors.append(f"SOURCE_SHA256_MISMATCH:{name}")
            if row.get("metadata_sha256") != sha256_file(metadata_path):
                errors.append(f"METADATA_SHA256_MISMATCH:{name}")
            expected_slides = len(Presentation(deck_path).slides)
            if row.get("slide_count") != expected_slides or metadata.get("slide_count") != expected_slides:
                errors.append(f"SLIDE_COUNT_MISMATCH:{name}")
            manifest_images = row.get("images")
            metadata_images = metadata.get("images")
            if manifest_images != metadata_images or not isinstance(manifest_images, list):
                errors.append(f"IMAGE_REPORT_CROSSLINK_MISMATCH:{name}")
                manifest_images = []
            actual_images = []
            for slide in range(1, expected_slides + 1):
                actual_images.append(validate_png(output_dir / name / f"Slide{slide}.PNG"))
            if manifest_images != actual_images:
                errors.append(f"IMAGE_SHA256_MISMATCH:{name}")
            expected_names = {f"Slide{slide}.PNG" for slide in range(1, expected_slides + 1)}
            actual_names = {path.name for path in (output_dir / name).glob("Slide*.PNG")}
            if actual_names != expected_names:
                errors.append(f"IMAGE_INVENTORY_MISMATCH:{name}")
            validated.append(name)
            image_count += len(actual_images)
            details.append({"name": name, "source_sha256": source_hash, "images": actual_images})
        expected_names = {path.stem for path in deck_paths}
        if set(by_name) != expected_names or manifest.get("deck_count") != len(deck_paths) or manifest.get("total_slide_count") != image_count:
            errors.append("MANIFEST_INVENTORY_MISMATCH")
        if errors:
            raise ValidationError(",".join(errors))
        return {
            "deck_count": len(validated),
            "slide_image_count": image_count,
            "validated_decks": validated,
            "batch_id": manifest["batch_id"],
            "report_sha256": recorded_report,
            "deck_details": details,
        }
    except (OSError, KeyError, TypeError, ValueError, ProvenanceError) as error:
        raise ValidationError(str(error)) from error


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--golden-set-dir", type=Path, default=Path("golden_set"))
    parser.add_argument("--output-dir", type=Path, default=Path("powerpoint_golden"))
    args = parser.parse_args()
    summary = validate_powerpoint_golden_batch(args.golden_set_dir, args.output_dir)
    print(f"Validated {summary['deck_count']} deck(s) and {summary['slide_image_count']} slide image(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
