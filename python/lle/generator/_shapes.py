"""Wall-shape data and placement logic shared by multiple generators."""

from __future__ import annotations

import random

WALL_SHAPES = (
    # (weight, offsets-from-anchor)
    (4, ((0, 0), (0, 1))),  # bar-2 horizontal
    (4, ((0, 0), (1, 0))),  # bar-2 vertical
    (1, ((0, 0), (0, 1), (0, 2))),  # bar-3 horizontal
    (1, ((0, 0), (1, 0), (2, 0))),  # bar-3 vertical
    (1, ((0, 0), (0, 1), (1, 0))),  # L
    (1, ((0, 0), (0, 1), (1, 1))),
    (1, ((0, 0), (1, 0), (1, 1))),
    (1, ((0, 1), (1, 0), (1, 1))),
    (2, ((0, 0), (0, 1), (1, 0), (1, 1))),  # 2x2 block
)

_WEIGHTS = [w for w, _ in WALL_SHAPES]
_SHAPES = [s for _, s in WALL_SHAPES]


def place_wall_shapes(
    free_cells: list[tuple[int, int]],
    budget: int,
    rng: random.Random,
) -> list[tuple[int, int]]:
    """Place walls as connected mini-shapes within a cell budget."""
    free_set = set(free_cells)
    anchors = list(free_cells)
    rng.shuffle(anchors)
    walls: list[tuple[int, int]] = []

    for anchor in anchors:
        if budget <= 0:
            break
        if anchor not in free_set:
            continue
        chosen_cells: list[tuple[int, int]] | None = None
        for shape in rng.choices(_SHAPES, weights=_WEIGHTS, k=4):
            if len(shape) > budget:
                continue
            cells = [(anchor[0] + dr, anchor[1] + dc) for dr, dc in shape]
            if all(cell in free_set for cell in cells):
                chosen_cells = cells
                break
        if chosen_cells is None:
            chosen_cells = [anchor]
        for cell in chosen_cells:
            free_set.discard(cell)
        walls.extend(chosen_cells)
        budget -= len(chosen_cells)

    return walls
