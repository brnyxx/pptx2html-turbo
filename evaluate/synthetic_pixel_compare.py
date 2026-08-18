from __future__ import annotations

import argparse
import json
import logging
from collections.abc import Sequence
from pathlib import Path
from typing import cast

from evaluate.strict_pixel_compare import (
    RgbaSlideComparison,
    StrictPixelError,
    compare_rgba_slide,
)
from evaluate.synthetic_scene import (
    CANVAS_HEIGHT_PX,
    CANVAS_WIDTH_PX,
    DECK_COUNT,
    SLIDES_PER_DECK,
)

logger = logging.getLogger(__name__)


class SyntheticPixelError(RuntimeError):
    pass


def compare_synthetic_batch(
    reference_dir: Path,
    candidate_dir: Path,
    *,
    diff_dir: Path | None = None,
    expected_deck_count: int | None = None,
    expected_slide_count: int | None = None,
    expected_resolution: tuple[int, int] | None = None,
) -> dict[str, object]:
    reference_dir = Path(reference_dir)
    candidate_dir = Path(candidate_dir)
    if not reference_dir.is_dir():
        raise SyntheticPixelError(
            f"SYNTHETIC_REFERENCE_ROOT_INVALID:{reference_dir}"
        )
    if not candidate_dir.is_dir():
        raise SyntheticPixelError(
            f"SYNTHETIC_CANDIDATE_ROOT_INVALID:{candidate_dir}"
        )
    reference_decks = sorted(path for path in reference_dir.iterdir() if path.is_dir())
    if not reference_decks:
        raise SyntheticPixelError(f"SYNTHETIC_REFERENCE_EMPTY:{reference_dir}")

    slides: list[RgbaSlideComparison] = []
    total_pixels = 0
    mismatched_pixels = 0
    max_channel_delta = 0
    output_resolution: list[int] | None = None
    expected_decks = {path.name for path in reference_decks}

    for reference_deck in reference_decks:
        deck_name = reference_deck.name
        candidate_deck = candidate_dir / deck_name
        reference_slides = sorted(
            reference_deck.glob("slide_*.png"),
            key=_slide_index,
        )
        if not reference_slides:
            raise SyntheticPixelError(
                f"SYNTHETIC_REFERENCE_DECK_EMPTY:{reference_deck}"
            )
        expected_files = {path.name for path in reference_slides}
        for reference_path in reference_slides:
            candidate_path = candidate_deck / reference_path.name
            if not candidate_path.is_file():
                raise SyntheticPixelError(
                    f"SYNTHETIC_CANDIDATE_MISSING:{candidate_path}"
                )
            slide_index = _slide_index(reference_path)
            try:
                slide = compare_rgba_slide(
                    deck_name,
                    slide_index,
                    reference_path,
                    candidate_path,
                    diff_dir,
                )
            except StrictPixelError as error:
                raise SyntheticPixelError(str(error)) from error
            if expected_resolution is not None and slide["size"] != list(
                expected_resolution
            ):
                expected_size = (
                    f"{expected_resolution[0]}x{expected_resolution[1]}"
                )
                actual_size = f"{slide['size'][0]}x{slide['size'][1]}"
                details = f"{deck_name}:{reference_path.name}:expected={expected_size}:actual={actual_size}"
                raise SyntheticPixelError(
                    f"SYNTHETIC_RESOLUTION_MISMATCH:{details}"
                )
            slides.append(slide)
            total_pixels += slide["total_pixels"]
            mismatched_pixels += slide["mismatched_pixels"]
            max_channel_delta = max(
                max_channel_delta,
                slide["max_channel_delta"],
            )
            if output_resolution is None:
                output_resolution = slide["size"].copy()

        candidate_files: set[str] = (
            {path.name for path in candidate_deck.glob("slide_*.png")}
            if candidate_deck.is_dir()
            else set()
        )
        extra_files = sorted(candidate_files - expected_files)
        if extra_files:
            extra_list = ",".join(extra_files)
            raise SyntheticPixelError(
                f"SYNTHETIC_CANDIDATE_EXTRA_SLIDES:{deck_name}:{extra_list}"
            )

    extra_decks = sorted(
        path.name
        for path in candidate_dir.iterdir()
        if path.is_dir() and path.name not in expected_decks
    )
    if extra_decks:
        raise SyntheticPixelError(
            f"SYNTHETIC_CANDIDATE_EXTRA_DECKS:{','.join(extra_decks)}"
        )
    deck_count = len(reference_decks)
    slide_count = len(slides)
    if expected_deck_count is not None and deck_count != expected_deck_count:
        details = f"expected={expected_deck_count}:actual={deck_count}"
        raise SyntheticPixelError(
            f"SYNTHETIC_DECK_COUNT_MISMATCH:{details}"
        )
    if expected_slide_count is not None and slide_count != expected_slide_count:
        details = f"expected={expected_slide_count}:actual={slide_count}"
        raise SyntheticPixelError(
            f"SYNTHETIC_SLIDE_COUNT_MISMATCH:{details}"
        )
    if expected_resolution is not None and output_resolution != list(
        expected_resolution
    ):
        expected_size = f"{expected_resolution[0]}x{expected_resolution[1]}"
        details = f"expected={expected_size}:actual={output_resolution}"
        raise SyntheticPixelError(
            f"SYNTHETIC_RESOLUTION_MISMATCH:{details}"
        )

    return {
        "ok": mismatched_pixels == 0,
        "comparison": "exact-rgba",
        "reference_oracle": "synthetic-scene-spec",
        "reference_platform": "Chromium",
        "output_resolution": output_resolution,
        "deck_count": deck_count,
        "slide_count": slide_count,
        "total_pixels": total_pixels,
        "mismatched_pixels": mismatched_pixels,
        "mismatch_ratio": (
            mismatched_pixels / total_pixels if total_pixels else 0.0
        ),
        "max_channel_delta": max_channel_delta,
        "slides": slides,
    }


def _slide_index(path: Path) -> int:
    try:
        return int(path.stem.removeprefix("slide_"))
    except ValueError as error:
        raise SyntheticPixelError(
            f"SYNTHETIC_SLIDE_NAME_INVALID:{path}"
        ) from error


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Require exact RGBA equality for the synthetic scene oracle."
    )
    _ = parser.add_argument("--reference-dir", type=Path, required=True)
    _ = parser.add_argument("--candidate-dir", type=Path, required=True)
    _ = parser.add_argument("--diff-dir", type=Path)
    _ = parser.add_argument("--output-json", type=Path, required=True)
    _ = parser.add_argument("--expected-decks", type=int, default=DECK_COUNT)
    _ = parser.add_argument(
        "--expected-slides",
        type=int,
        default=DECK_COUNT * SLIDES_PER_DECK,
    )
    _ = parser.add_argument("--width", type=int, default=CANVAS_WIDTH_PX)
    _ = parser.add_argument("--height", type=int, default=CANVAS_HEIGHT_PX)
    args = parser.parse_args(argv)
    reference_dir = cast(Path, args.reference_dir)
    candidate_dir = cast(Path, args.candidate_dir)
    diff_dir = cast(Path | None, args.diff_dir)
    output_json = cast(Path, args.output_json)
    expected_decks = cast(int, args.expected_decks)
    expected_slides = cast(int, args.expected_slides)
    width = cast(int, args.width)
    height = cast(int, args.height)

    try:
        result = compare_synthetic_batch(
            reference_dir,
            candidate_dir,
            diff_dir=diff_dir,
            expected_deck_count=expected_decks,
            expected_slide_count=expected_slides,
            expected_resolution=(width, height),
        )
    except SyntheticPixelError as error:
        result = {"ok": False, "error": str(error)}
    output_json.parent.mkdir(parents=True, exist_ok=True)
    _ = output_json.write_text(
        json.dumps(result, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    logger.info("Synthetic strict comparison report: %s", output_json)
    return 0 if result["ok"] else 1


if __name__ == "__main__":
    logging.basicConfig(level=logging.INFO, format="%(levelname)s: %(message)s")
    raise SystemExit(main())
