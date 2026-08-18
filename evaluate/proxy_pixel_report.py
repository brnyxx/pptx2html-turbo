"""Measure deterministic pixel similarity against a non-native proxy renderer."""

from __future__ import annotations

import argparse
import json
import logging
from pathlib import Path
from typing import Any

import numpy as np
from PIL import Image

LOGGER = logging.getLogger(__name__)
ASPECT_RATIO_TOLERANCE = 0.001


def _load_rgb(path: Path) -> Image.Image:
    with Image.open(path) as image:
        return image.convert("RGB")


def compare_pair(
    candidate_path: Path,
    reference_path: Path,
    *,
    allow_reference_resize: bool = False,
) -> dict[str, Any]:
    candidate = _load_rgb(candidate_path)
    reference = _load_rgb(reference_path)
    reference_resized = False

    if candidate.size != reference.size:
        if not allow_reference_resize:
            raise ValueError(
                f"Image dimensions differ: candidate={candidate.size} "
                f"reference={reference.size}"
            )
        candidate_ratio = candidate.width / candidate.height
        reference_ratio = reference.width / reference.height
        relative_error = abs(candidate_ratio - reference_ratio) / candidate_ratio
        if relative_error > ASPECT_RATIO_TOLERANCE:
            raise ValueError(
                f"Image dimensions have different aspect ratios: "
                f"candidate={candidate.size} reference={reference.size}"
            )
        reference = reference.resize(candidate.size, Image.Resampling.LANCZOS)
        reference_resized = True

    candidate_pixels = np.asarray(candidate, dtype=np.int16)
    reference_pixels = np.asarray(reference, dtype=np.int16)
    delta = np.abs(candidate_pixels - reference_pixels)
    pixel_mismatch = np.any(delta != 0, axis=2)
    pixel_count = int(pixel_mismatch.size)
    mismatched_pixels = int(np.count_nonzero(pixel_mismatch))
    mae = float(delta.mean())

    return {
        "candidate": str(candidate_path),
        "reference": str(reference_path),
        "width": candidate.width,
        "height": candidate.height,
        "reference_resized": reference_resized,
        "mismatched_pixels": mismatched_pixels,
        "mismatched_pixel_ratio": mismatched_pixels / pixel_count,
        "mae": mae,
        "max_channel_delta": int(delta.max()),
        "similarity": 100.0 * (1.0 - mae / 255.0),
    }


def create_report(
    candidate_root: Path,
    reference_root: Path,
    *,
    allow_reference_resize: bool = False,
) -> dict[str, Any]:
    candidate_paths = sorted(candidate_root.rglob("*.png"))
    if not candidate_paths:
        raise ValueError(f"No candidate PNGs found under {candidate_root}")

    candidate_relatives = {path.relative_to(candidate_root) for path in candidate_paths}
    reference_relatives = {
        path.relative_to(reference_root) for path in reference_root.rglob("*.png")
    }
    missing = sorted(candidate_relatives - reference_relatives)
    extra = sorted(reference_relatives - candidate_relatives)
    if missing or extra:
        raise ValueError(f"Candidate/reference slide mismatch: missing={missing} extra={extra}")

    slides = []
    for candidate_path in candidate_paths:
        relative = candidate_path.relative_to(candidate_root)
        result = compare_pair(
            candidate_path,
            reference_root / relative,
            allow_reference_resize=allow_reference_resize,
        )
        result["slide"] = relative.as_posix()
        slides.append(result)

    similarities = [slide["similarity"] for slide in slides]
    mismatch_ratios = [slide["mismatched_pixel_ratio"] for slide in slides]
    return {
        "slide_count": len(slides),
        "corpus_similarity": float(np.mean(similarities)),
        "minimum_slide_similarity": min(similarities),
        "mean_mismatched_pixel_ratio": float(np.mean(mismatch_ratios)),
        "maximum_channel_delta": max(slide["max_channel_delta"] for slide in slides),
        "all_slides_meet_95_percent": all(score >= 95.0 for score in similarities),
        "slides": slides,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--candidate-root", type=Path, required=True)
    parser.add_argument("--reference-root", type=Path, required=True)
    parser.add_argument("--allow-reference-resize", action="store_true")
    parser.add_argument("--json-output", type=Path)
    args = parser.parse_args()

    logging.basicConfig(level=logging.INFO, format="%(levelname)s: %(message)s")
    report = create_report(
        args.candidate_root,
        args.reference_root,
        allow_reference_resize=args.allow_reference_resize,
    )
    rendered = json.dumps(report, indent=2, sort_keys=True)
    if args.json_output is not None:
        args.json_output.parent.mkdir(parents=True, exist_ok=True)
        args.json_output.write_text(f"{rendered}\n", encoding="utf-8")
    LOGGER.info(
        "Slides: %d, corpus similarity: %.6f%%, minimum: %.6f%%, all >=95%%: %s",
        report["slide_count"],
        report["corpus_similarity"],
        report["minimum_slide_similarity"],
        report["all_slides_meet_95_percent"],
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
