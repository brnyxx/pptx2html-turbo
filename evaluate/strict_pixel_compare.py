from __future__ import annotations

import argparse
import json
from collections.abc import Sequence
from pathlib import Path
from typing import TypedDict

from PIL import Image, ImageChops, ImageMath

try:
    from evaluate.validate_powerpoint_golden import (
        ValidationError,
        validate_powerpoint_golden_batch,
    )
except ModuleNotFoundError:
    from validate_powerpoint_golden import (
        ValidationError,
        validate_powerpoint_golden_batch,
    )


class StrictPixelError(RuntimeError):
    pass


class RgbaSlideComparison(TypedDict):
    deck: str
    slide_index: int
    reference: str
    candidate: str
    size: list[int]
    total_pixels: int
    mismatched_pixels: int
    mismatch_ratio: float
    max_channel_delta: int
    exact: bool
    diff: str | None


def compare_strict_batch(
    golden_set_dir: Path,
    reference_dir: Path,
    candidate_dir: Path,
    *,
    diff_dir: Path | None = None,
) -> dict[str, object]:
    golden_set_dir = Path(golden_set_dir)
    reference_dir = Path(reference_dir)
    candidate_dir = Path(candidate_dir)
    try:
        evidence = validate_powerpoint_golden_batch(golden_set_dir, reference_dir)
    except ValidationError as error:
        raise StrictPixelError(f"PIXEL_REFERENCE_INVALID:{error}") from error

    manifest = json.loads(
        (reference_dir / "manifest.json").read_text(encoding="utf-8")
    )
    slides: list[RgbaSlideComparison] = []
    total_pixels = 0
    mismatched_pixels = 0
    max_channel_delta = 0

    for deck in manifest["decks"]:
        deck_name = deck["name"]
        for slide_index, image in enumerate(deck["images"]):
            reference_path = reference_dir / deck["output_dir"] / image["file"]
            candidate_path = candidate_dir / deck_name / f"slide_{slide_index}.png"
            if not candidate_path.is_file():
                raise StrictPixelError(
                    f"PIXEL_CANDIDATE_MISSING:{candidate_path.as_posix()}"
                )
            slide = compare_rgba_slide(
                deck_name,
                slide_index,
                reference_path,
                candidate_path,
                diff_dir,
            )
            slides.append(slide)
            total_pixels += slide["total_pixels"]
            mismatched_pixels += slide["mismatched_pixels"]
            max_channel_delta = max(max_channel_delta, slide["max_channel_delta"])

    expected_decks = {deck["name"] for deck in manifest["decks"]}
    extra_decks = sorted(
        path.name
        for path in candidate_dir.iterdir()
        if path.is_dir() and path.name not in expected_decks
    )
    if extra_decks:
        raise StrictPixelError(f"PIXEL_CANDIDATE_EXTRA_DECKS:{','.join(extra_decks)}")

    return {
        "ok": mismatched_pixels == 0,
        "comparison": "exact-rgba",
        "reference_oracle": "Microsoft PowerPoint",
        "reference_platform": "Windows",
        "output_resolution": manifest["output_resolution"],
        "batch_id": manifest["batch_id"],
        "golden_set_revision": manifest["golden_set_revision"],
        "deck_count": evidence["deck_count"],
        "slide_count": evidence["slide_image_count"],
        "total_pixels": total_pixels,
        "mismatched_pixels": mismatched_pixels,
        "mismatch_ratio": (
            mismatched_pixels / total_pixels if total_pixels else 0.0
        ),
        "max_channel_delta": max_channel_delta,
        "slides": slides,
    }


def compare_rgba_slide(
    deck_name: str,
    slide_index: int,
    reference_path: Path,
    candidate_path: Path,
    diff_dir: Path | None,
) -> RgbaSlideComparison:
    with Image.open(reference_path) as reference_source:
        reference = reference_source.convert("RGBA")
    with Image.open(candidate_path) as candidate_source:
        candidate = candidate_source.convert("RGBA")

    if reference.size != candidate.size:
        raise StrictPixelError(
            "PIXEL_DIMENSION_MISMATCH:"
            f"{deck_name}:slide_{slide_index}:"
            f"reference={reference.size[0]}x{reference.size[1]}:"
            f"candidate={candidate.size[0]}x{candidate.size[1]}"
        )

    difference = ImageChops.difference(reference, candidate)
    extrema = difference.getextrema()
    max_delta = max(channel[1] for channel in extrema)
    red, green, blue, alpha = difference.split()
    mismatch_mask = ImageMath.unsafe_eval(
        "max(max(red, green), max(blue, alpha))",
        red=red,
        green=green,
        blue=blue,
        alpha=alpha,
    ).convert("L").point(
        lambda value: 255 if value else 0
    )
    histogram = mismatch_mask.histogram()
    mismatched = sum(histogram[1:])
    total = reference.width * reference.height

    diff_path = None
    if mismatched and diff_dir is not None:
        diff_path = Path(diff_dir) / deck_name / f"Slide{slide_index + 1}.PNG"
        diff_path.parent.mkdir(parents=True, exist_ok=True)
        difference.save(diff_path)

    return {
        "deck": deck_name,
        "slide_index": slide_index,
        "reference": reference_path.as_posix(),
        "candidate": candidate_path.as_posix(),
        "size": [reference.width, reference.height],
        "total_pixels": total,
        "mismatched_pixels": mismatched,
        "mismatch_ratio": mismatched / total if total else 0.0,
        "max_channel_delta": max_delta,
        "exact": mismatched == 0,
        "diff": diff_path.as_posix() if diff_path is not None else None,
    }


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Require exact RGBA equality with PowerPoint-native slide PNGs."
    )
    parser.add_argument("--golden-set-dir", type=Path, required=True)
    parser.add_argument("--reference-dir", type=Path, required=True)
    parser.add_argument("--candidate-dir", type=Path, required=True)
    parser.add_argument("--diff-dir", type=Path)
    parser.add_argument("--output-json", type=Path)
    args = parser.parse_args(argv)

    try:
        result = compare_strict_batch(
            args.golden_set_dir,
            args.reference_dir,
            args.candidate_dir,
            diff_dir=args.diff_dir,
        )
    except StrictPixelError as error:
        result = {"ok": False, "error": str(error)}

    payload = json.dumps(result, indent=2, sort_keys=True) + "\n"
    if args.output_json is not None:
        args.output_json.parent.mkdir(parents=True, exist_ok=True)
        args.output_json.write_text(payload, encoding="utf-8")
    print(payload, end="")
    return 0 if result["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
