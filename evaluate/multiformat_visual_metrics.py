from __future__ import annotations

from decimal import Decimal
from pathlib import Path

import numpy as np
from numpy.typing import NDArray
from PIL import Image

from evaluate.multiformat_inventory_types import Box
from evaluate.multiformat_metric_types import (
    MetricError,
    VisualScores,
    retained_decimal,
)
from evaluate.multiformat_visual_color_edges import (
    color_similarity,
    edge_f1,
)
from evaluate.multiformat_visual_ssim import (
    active_ssim,
    active_tile_mask,
    multiscale_ssim,
)

FloatImage = NDArray[np.float64]


def score_visual(
    reference_path: Path,
    candidate_path: Path,
    background: str,
    oracle_boxes: tuple[Box, ...],
) -> VisualScores:
    reference_linear, reference_srgb = _load_png(reference_path, background)
    candidate_linear, candidate_srgb = _load_png(candidate_path, background)
    if reference_linear.shape != candidate_linear.shape:
        raise MetricError("artifact.dimension", "native PNG dimensions differ")
    active = active_tile_mask(reference_linear, candidate_linear, oracle_boxes)
    return VisualScores(
        _decimal(multiscale_ssim(reference_linear, candidate_linear)),
        _decimal(active_ssim(reference_linear, candidate_linear, active)),
        _decimal(color_similarity(reference_srgb, candidate_srgb, active)),
        _decimal(edge_f1(reference_linear, candidate_linear)),
    )


def png_dimensions(path: Path) -> tuple[int, int]:
    try:
        with Image.open(path) as image:
            if image.format != "PNG" or image.mode not in {"RGB", "RGBA"}:
                raise MetricError("artifact.png", path.as_posix())
            return image.size
    except (OSError, ValueError) as error:
        raise MetricError("artifact.png", path.as_posix()) from error


def _load_png(path: Path, background: str) -> tuple[FloatImage, FloatImage]:
    try:
        with Image.open(path) as image:
            if image.format != "PNG" or image.mode not in {"RGB", "RGBA"}:
                raise MetricError("artifact.png", path.as_posix())
            rgba = np.asarray(image.convert("RGBA"), dtype=np.float64) / 255.0
    except (OSError, ValueError) as error:
        raise MetricError("artifact.png", path.as_posix()) from error
    background_srgb = np.array(_parse_background(background), dtype=np.float64)
    rgb_linear = _srgb_to_linear(rgba[:, :, :3])
    background_linear = _srgb_to_linear(background_srgb)
    alpha = rgba[:, :, 3:4]
    composited_linear = rgb_linear * alpha + background_linear * (1.0 - alpha)
    composited_srgb = _linear_to_srgb(composited_linear)
    return composited_linear, composited_srgb


def _parse_background(value: str) -> tuple[float, float, float]:
    if len(value) != 7 or not value.startswith("#"):
        raise MetricError("inventory.background", value)
    try:
        channels = tuple(int(value[index : index + 2], 16) / 255 for index in (1, 3, 5))
    except ValueError as error:
        raise MetricError("inventory.background", value) from error
    return channels


def _srgb_to_linear(value: FloatImage) -> FloatImage:
    return np.where(
        value <= 0.04045,
        value / 12.92,
        ((value + 0.055) / 1.055) ** 2.4,
    )


def _linear_to_srgb(value: FloatImage) -> FloatImage:
    return np.where(
        value <= 0.0031308,
        12.92 * value,
        1.055 * np.power(value, 1 / 2.4) - 0.055,
    )


def _decimal(value: float) -> Decimal:
    bounded = max(0.0, min(100.0, value))
    return retained_decimal(Decimal(str(bounded)))
