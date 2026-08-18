from __future__ import annotations

import argparse
from collections.abc import Sequence
from html import escape
from pathlib import Path
from typing import cast

from evaluate.synthetic_scene import (
    CANVAS_HEIGHT_PX,
    CANVAS_WIDTH_PX,
    EMU_PER_PIXEL,
    SyntheticDeck,
    SyntheticScene,
    create_synthetic_corpus,
)

_DOCUMENT_START = f"""<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<style>
* {{ margin:0; padding:0; box-sizing:border-box; }}
html, body {{ margin:0; padding:0; }}
body {{ background:#f0f0f0; }}
.synthetic-container {{ display:flex; flex-direction:column; align-items:safe center; gap:20px; padding:20px; }}
.slide-shell {{ position:relative; flex:0 0 auto; overflow:hidden; }}
.slide {{ position:relative; width:{CANVAS_WIDTH_PX:.1f}px; height:{CANVAS_HEIGHT_PX:.1f}px; overflow:hidden; }}
.synthetic-rectangle {{ position:absolute; }}
</style>
</head>
<body>
<div class="synthetic-container">
"""
_DOCUMENT_END = """</div>
</body>
</html>
"""


def write_synthetic_reference_corpus(
    corpus: Sequence[SyntheticDeck],
    output_dir: Path,
) -> tuple[Path, ...]:
    output_dir.mkdir(parents=True, exist_ok=True)
    outputs: list[Path] = []
    for deck in corpus:
        html = [_DOCUMENT_START]
        html.extend(_scene_html(scene) for scene in deck.scenes)
        html.append(_DOCUMENT_END)
        output = output_dir / f"{deck.name}.html"
        _ = output.write_text("".join(html), encoding="utf-8")
        outputs.append(output)
    return tuple(outputs)


def _scene_html(scene: SyntheticScene) -> str:
    parts = [
        '<div class="slide-shell">',
        (
            f'<div class="slide" data-scene-id="{escape(scene.scene_id)}" '
            f'style="background:#{scene.background}">'
        ),
    ]
    for rectangle in scene.rectangles:
        style = "".join(
            (
                f"left:{rectangle.x / EMU_PER_PIXEL:.1f}px;",
                f"top:{rectangle.y / EMU_PER_PIXEL:.1f}px;",
                f"width:{rectangle.width / EMU_PER_PIXEL:.1f}px;",
                f"height:{rectangle.height / EMU_PER_PIXEL:.1f}px;",
                f"background:#{rectangle.fill}",
            )
        )
        parts.append(
            f'<div class="synthetic-rectangle" style="{style}"></div>'
        )
    parts.append("</div></div>\n")
    return "".join(parts)


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Generate independent synthetic reference HTML."
    )
    _ = parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args(argv)

    output = cast(Path, args.output)
    _ = write_synthetic_reference_corpus(create_synthetic_corpus(), output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
