from __future__ import annotations

import numpy as np
from numpy.typing import NDArray
from scipy.ndimage import gaussian_filter
from skimage.feature import canny

from evaluate.multiformat_inventory_types import Box
from evaluate.multiformat_metric_types import MetricError

FloatImage = NDArray[np.float64]
BoolImage = NDArray[np.bool_]
MS_WEIGHTS = np.array([0.0448, 0.2856, 0.3001, 0.2363, 0.1333])
WINDOW_TRUNCATE = 5.0 / 1.5
C1 = 0.01**2
C2 = 0.03**2


def multiscale_ssim(reference: FloatImage, candidate: FloatImage) -> float:
    left = reference
    right = candidate
    contrast_values: list[float] = []
    final_ssim = 0.0
    for scale in range(5):
        if min(left.shape[:2]) < 11:
            raise MetricError(
                "artifact.dimension", "image is too small for five scales"
            )
        luminance_map, contrast_map = _ssim_maps(left, right)
        final_ssim = float(np.mean(luminance_map * contrast_map))
        contrast_values.append(float(np.mean(contrast_map)))
        if scale < 4:
            left = _downsample(left)
            right = _downsample(right)
    factors = [
        max(0.0, min(1.0, contrast_values[index])) ** MS_WEIGHTS[index]
        for index in range(4)
    ]
    factors.append(max(0.0, min(1.0, final_ssim)) ** MS_WEIGHTS[4])
    return float(np.prod(factors)) * 100.0


def active_tile_mask(
    reference: FloatImage,
    candidate: FloatImage,
    oracle_boxes: tuple[Box, ...],
) -> BoolImage:
    height, width = reference.shape[:2]
    left_gray = _gray8(reference)
    right_gray = _gray8(candidate)
    edges = canny(
        left_gray / 255,
        sigma=1.0,
        low_threshold=100 / 255,
        high_threshold=200 / 255,
    )
    edges |= canny(
        right_gray / 255,
        sigma=1.0,
        low_threshold=100 / 255,
        high_threshold=200 / 255,
    )
    box_mask = _box_mask(height, width, oracle_boxes)
    active = np.zeros((height, width), dtype=np.bool_)
    row_edges = np.linspace(0, height, 33, dtype=np.int64)
    column_edges = np.linspace(0, width, 33, dtype=np.int64)
    for row in range(32):
        top, bottom = row_edges[row], row_edges[row + 1]
        for column in range(32):
            left, right = column_edges[column], column_edges[column + 1]
            if top == bottom or left == right:
                continue
            left_tile = left_gray[top:bottom, left:right]
            right_tile = right_gray[top:bottom, left:right]
            if (
                float(np.var(left_tile)) > 16
                or float(np.var(right_tile)) > 16
                or bool(np.any(edges[top:bottom, left:right]))
                or bool(np.any(box_mask[top:bottom, left:right]))
            ):
                active[top:bottom, left:right] = True
    return active


def active_ssim(
    reference: FloatImage,
    candidate: FloatImage,
    mask: BoolImage,
) -> float:
    if not bool(np.any(mask)):
        return 100.0
    luminance, contrast = _ssim_maps(reference, candidate)
    score_map = np.mean(luminance * contrast, axis=2)
    return float(np.mean(score_map[mask])) * 100.0


def _ssim_maps(
    reference: FloatImage,
    candidate: FloatImage,
) -> tuple[FloatImage, FloatImage]:
    mu_left = gaussian_filter(
        reference,
        sigma=(1.5, 1.5, 0),
        truncate=WINDOW_TRUNCATE,
        mode="reflect",
    )
    mu_right = gaussian_filter(
        candidate,
        sigma=(1.5, 1.5, 0),
        truncate=WINDOW_TRUNCATE,
        mode="reflect",
    )
    variance_left = (
        gaussian_filter(
            reference * reference,
            sigma=(1.5, 1.5, 0),
            truncate=WINDOW_TRUNCATE,
            mode="reflect",
        )
        - mu_left * mu_left
    )
    variance_right = (
        gaussian_filter(
            candidate * candidate,
            sigma=(1.5, 1.5, 0),
            truncate=WINDOW_TRUNCATE,
            mode="reflect",
        )
        - mu_right * mu_right
    )
    covariance = (
        gaussian_filter(
            reference * candidate,
            sigma=(1.5, 1.5, 0),
            truncate=WINDOW_TRUNCATE,
            mode="reflect",
        )
        - mu_left * mu_right
    )
    luminance = (2 * mu_left * mu_right + C1) / (
        mu_left * mu_left + mu_right * mu_right + C1
    )
    contrast = (2 * covariance + C2) / (variance_left + variance_right + C2)
    return np.clip(luminance, 0, 1), np.clip(contrast, 0, 1)


def _downsample(image: FloatImage) -> FloatImage:
    height = image.shape[0] - image.shape[0] % 2
    width = image.shape[1] - image.shape[1] % 2
    cropped = image[:height, :width]
    return (
        cropped[0::2, 0::2]
        + cropped[1::2, 0::2]
        + cropped[0::2, 1::2]
        + cropped[1::2, 1::2]
    ) / 4


def _gray(image: FloatImage) -> FloatImage:
    return (
        DecimalWeights.RED * image[:, :, 0]
        + DecimalWeights.GREEN * image[:, :, 1]
        + DecimalWeights.BLUE * image[:, :, 2]
    )


def _gray8(image: FloatImage) -> NDArray[np.uint8]:
    return np.rint(_gray(image) * 255).clip(0, 255).astype(np.uint8)


class DecimalWeights:
    RED = 0.2126
    GREEN = 0.7152
    BLUE = 0.0722


def _box_mask(height: int, width: int, boxes: tuple[Box, ...]) -> BoolImage:
    mask = np.zeros((height, width), dtype=np.bool_)
    for box in boxes:
        left = max(0, min(width, int(float(box.x))))
        top = max(0, min(height, int(float(box.y))))
        right = max(left, min(width, int(float(box.x) + float(box.width))))
        bottom = max(top, min(height, int(float(box.y) + float(box.height))))
        mask[top:bottom, left:right] = True
    return mask
