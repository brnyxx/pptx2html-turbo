from __future__ import annotations

from collections import deque

import numpy as np
from numpy.typing import NDArray
from skimage.color import deltaE_ciede2000, rgb2lab
from skimage.feature import canny

FloatImage = NDArray[np.float64]
BoolImage = NDArray[np.bool_]


def color_similarity(
    reference_srgb: FloatImage,
    candidate_srgb: FloatImage,
    active: BoolImage,
) -> float:
    if not bool(np.any(active)):
        return 100.0
    reference_lab = rgb2lab(reference_srgb)
    candidate_lab = rgb2lab(candidate_srgb)
    differences = deltaE_ciede2000(reference_lab, candidate_lab)
    bounded = np.minimum(differences[active], 20.0)
    return 100.0 * (1.0 - float(np.mean(bounded)) / 20.0)


def edge_f1(reference: FloatImage, candidate: FloatImage) -> float:
    reference_edges = canny(
        _gray8(reference) / 255,
        sigma=1.0,
        low_threshold=100 / 255,
        high_threshold=200 / 255,
    )
    candidate_edges = canny(
        _gray8(candidate) / 255,
        sigma=1.0,
        low_threshold=100 / 255,
        high_threshold=200 / 255,
    )
    left = [tuple(value) for value in np.argwhere(reference_edges)]
    right = [tuple(value) for value in np.argwhere(candidate_edges)]
    if not left and not right:
        return 100.0
    if not left or not right:
        return 0.0
    matches = _maximum_matching(left, right)
    return 200.0 * matches / (len(left) + len(right))


def _maximum_matching(
    left: list[tuple[int, int]],
    right: list[tuple[int, int]],
) -> int:
    right_by_coordinate: dict[tuple[int, int], list[int]] = {}
    for index, coordinate in enumerate(right):
        right_by_coordinate.setdefault(coordinate, []).append(index)
    adjacency: list[list[int]] = []
    for row, column in left:
        candidates: list[int] = []
        for row_delta in (-1, 0, 1):
            for column_delta in (-1, 0, 1):
                candidates.extend(
                    right_by_coordinate.get(
                        (row + row_delta, column + column_delta),
                        [],
                    )
                )
        adjacency.append(candidates)
    return _hopcroft_karp(adjacency, len(right))


def _hopcroft_karp(adjacency: list[list[int]], right_count: int) -> int:
    left_match = [-1] * len(adjacency)
    right_match = [-1] * right_count
    distance = [0] * len(adjacency)

    def breadth_first() -> bool:
        queue: deque[int] = deque()
        found = False
        for left_index, match in enumerate(left_match):
            if match < 0:
                distance[left_index] = 0
                queue.append(left_index)
            else:
                distance[left_index] = -1
        while queue:
            left_index = queue.popleft()
            for right_index in adjacency[left_index]:
                paired = right_match[right_index]
                if paired < 0:
                    found = True
                elif distance[paired] < 0:
                    distance[paired] = distance[left_index] + 1
                    queue.append(paired)
        return found

    def depth_first(left_index: int) -> bool:
        for right_index in adjacency[left_index]:
            paired = right_match[right_index]
            if paired < 0 or (
                distance[paired] == distance[left_index] + 1 and depth_first(paired)
            ):
                left_match[left_index] = right_index
                right_match[right_index] = left_index
                return True
        distance[left_index] = -1
        return False

    matches = 0
    while breadth_first():
        for left_index, match in enumerate(left_match):
            if match < 0 and depth_first(left_index):
                matches += 1
    return matches


def _gray(image: FloatImage) -> FloatImage:
    return 0.2126 * image[:, :, 0] + 0.7152 * image[:, :, 1] + 0.0722 * image[:, :, 2]


def _gray8(image: FloatImage) -> NDArray[np.uint8]:
    return np.rint(_gray(image) * 255).clip(0, 255).astype(np.uint8)
