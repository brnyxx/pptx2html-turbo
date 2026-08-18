from __future__ import annotations

from dataclasses import dataclass
from typing import Final, NewType

Emu = NewType("Emu", int)
RgbHex = NewType("RgbHex", str)

EMU_PER_PIXEL: Final = 9_525
CANVAS_WIDTH_PX: Final = 960
CANVAS_HEIGHT_PX: Final = 540
CANVAS_WIDTH_EMU: Final = CANVAS_WIDTH_PX * EMU_PER_PIXEL
CANVAS_HEIGHT_EMU: Final = CANVAS_HEIGHT_PX * EMU_PER_PIXEL
DECK_COUNT: Final = 10
SLIDES_PER_DECK: Final = 10
RECTANGLES_PER_SLIDE: Final = 8

_PALETTES: Final = (
    ("F7F9FC", "234E70", "FBF8BE", "1B998B", "F46036", "6A4C93", "3A86FF"),
    ("FFF8F0", "5F0F40", "9A031E", "FB8B24", "E36414", "0F4C5C", "2A9D8F"),
    ("F5F3FF", "240046", "5A189A", "9D4EDD", "E0AAFF", "FF6D00", "FF9E00"),
    ("F2FAF7", "003049", "D62828", "F77F00", "FCBF49", "2A9D8F", "264653"),
    ("FFFDF2", "386641", "6A994E", "A7C957", "F2E8CF", "BC4749", "8C1C13"),
    ("F5FBFF", "03045E", "0077B6", "00B4D8", "90E0EF", "FFB703", "FB8500"),
    ("FFF7FB", "590D22", "800F2F", "A4133C", "C9184A", "FF4D6D", "FFB3C1"),
    ("F5FFFC", "05668D", "028090", "00A896", "02C39A", "F0F3BD", "F25F5C"),
    ("F8F7FF", "22223B", "4A4E69", "9A8C98", "C9ADA7", "F2E9E4", "3A86FF"),
    ("FFFBF5", "2B2D42", "8D99AE", "EDF2F4", "EF233C", "D90429", "F4A261"),
)


@dataclass(frozen=True, slots=True)
class SolidRectangle:
    x: Emu
    y: Emu
    width: Emu
    height: Emu
    fill: RgbHex


@dataclass(frozen=True, slots=True)
class SyntheticScene:
    scene_id: str
    background: RgbHex
    rectangles: tuple[SolidRectangle, ...]


@dataclass(frozen=True, slots=True)
class SyntheticDeck:
    name: str
    scenes: tuple[SyntheticScene, ...]


def create_synthetic_corpus() -> tuple[SyntheticDeck, ...]:
    decks: list[SyntheticDeck] = []
    for deck_index in range(DECK_COUNT):
        palette = _PALETTES[deck_index]
        scenes: list[SyntheticScene] = []
        for slide_index in range(SLIDES_PER_DECK):
            rectangles: list[SolidRectangle] = []
            for rectangle_index in range(RECTANGLES_PER_SLIDE):
                seed = (
                    (deck_index + 1) * 104_729
                    + (slide_index + 1) * 13_007
                    + (rectangle_index + 1) * 1_009
                )
                width_px = 52 + seed % 181
                height_px = 34 + (seed // 7) % 123
                x_px = (seed // 11) % (CANVAS_WIDTH_PX - width_px + 1)
                y_px = (seed // 17) % (CANVAS_HEIGHT_PX - height_px + 1)
                rectangles.append(
                    SolidRectangle(
                        x=Emu(x_px * EMU_PER_PIXEL),
                        y=Emu(y_px * EMU_PER_PIXEL),
                        width=Emu(width_px * EMU_PER_PIXEL),
                        height=Emu(height_px * EMU_PER_PIXEL),
                        fill=RgbHex(
                            palette[
                                1
                                + (slide_index + rectangle_index)
                                % (len(palette) - 1)
                            ]
                        ),
                    )
                )
            scenes.append(
                SyntheticScene(
                    scene_id=(
                        f"synthetic_{deck_index + 1:02d}/"
                        f"slide_{slide_index:02d}"
                    ),
                    background=RgbHex(palette[0]),
                    rectangles=tuple(rectangles),
                )
            )
        decks.append(
            SyntheticDeck(
                name=f"synthetic_{deck_index + 1:02d}",
                scenes=tuple(scenes),
            )
        )
    return tuple(decks)
