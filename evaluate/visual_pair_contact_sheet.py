from __future__ import annotations

import argparse
import logging
import math
from pathlib import Path

from PIL import Image, ImageDraw, ImageOps

logger = logging.getLogger(__name__)

LABEL_HEIGHT = 18


def build_contact_sheets(
    reference_dir: Path,
    candidate_dir: Path,
    output_dir: Path,
    *,
    thumbnail_width: int = 240,
    thumbnail_height: int = 135,
    pairs_per_row: int = 2,
) -> list[Path]:
    output_dir.mkdir(parents=True, exist_ok=True)
    sheets: list[Path] = []

    for reference_deck in sorted(path for path in reference_dir.iterdir() if path.is_dir()):
        reference_paths = sorted(reference_deck.glob("slide_*.png"))
        if not reference_paths:
            continue
        candidate_deck = candidate_dir / reference_deck.name
        candidate_paths = [
            candidate_deck / reference_path.name
            for reference_path in reference_paths
        ]
        missing = [
            path.relative_to(candidate_dir)
            for path in candidate_paths
            if not path.exists()
        ]
        if missing:
            raise FileNotFoundError(
                "CANDIDATE_SLIDE_MISSING: "
                + ", ".join(str(path) for path in missing)
            )

        rows = math.ceil(len(reference_paths) / pairs_per_row)
        cell_height = thumbnail_height + LABEL_HEIGHT
        sheet = Image.new(
            "RGB",
            (
                thumbnail_width * pairs_per_row * 2,
                cell_height * rows,
            ),
            "white",
        )
        draw = ImageDraw.Draw(sheet)
        for index, (reference_path, candidate_path) in enumerate(
            zip(reference_paths, candidate_paths, strict=True)
        ):
            with Image.open(reference_path) as reference_source:
                reference_size = reference_source.size
            with Image.open(candidate_path) as candidate_source:
                candidate_size = candidate_source.size
            aspect_delta = abs(
                reference_size[0] * candidate_size[1]
                - reference_size[1] * candidate_size[0]
            )
            if aspect_delta > max(*reference_size, *candidate_size):
                raise ValueError(
                    "VISUAL_CAPTURE_ASPECT_MISMATCH: "
                    f"{reference_path} is {reference_size[0]}x{reference_size[1]}"
                )
            row, pair_column = divmod(index, pairs_per_row)
            for image_column, (role, path) in enumerate(
                [
                    ("REF", reference_path),
                    ("HTML", candidate_path),
                ]
            ):
                with Image.open(path) as source:
                    image = source.convert("RGB")
                    thumbnail = ImageOps.contain(
                        image,
                        (thumbnail_width, thumbnail_height),
                        Image.Resampling.LANCZOS,
                    )
                x = (pair_column * 2 + image_column) * thumbnail_width
                y = row * cell_height
                thumbnail_x = x + (thumbnail_width - thumbnail.width) // 2
                thumbnail_y = y + (thumbnail_height - thumbnail.height) // 2
                sheet.paste(thumbnail, (thumbnail_x, thumbnail_y))
                draw.text(
                    (x + 4, y + thumbnail_height + 2),
                    f"{role} {reference_path.stem}",
                    fill="black",
                )

        output_path = output_dir / f"{reference_deck.name}.png"
        sheet.save(output_path)
        sheets.append(output_path)

    return sheets


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Build labeled reference/candidate contact sheets"
    )
    parser.add_argument("--references", type=Path, required=True)
    parser.add_argument("--candidates", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    sheets = build_contact_sheets(
        args.references,
        args.candidates,
        args.output,
    )
    logger.info("Built %d contact sheets in %s", len(sheets), args.output)


if __name__ == "__main__":
    logging.basicConfig(level=logging.INFO, format="%(levelname)s: %(message)s")
    main()
