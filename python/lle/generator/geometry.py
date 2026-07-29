"""Grid geometry helpers and wall-shape utilities used by generators."""

from __future__ import annotations

import random

from lle.tiles import Direction
from lle.types import Position


def in_bounds(pos: Position, rows: int, cols: int) -> bool:
    r, c = pos
    return 0 <= r < rows and 0 <= c < cols


def points_out_immediately(src: Position, direction: Direction, rows: int, cols: int) -> bool:
    dr, dc = direction.delta
    nr, nc = src[0] + dr, src[1] + dc
    return not in_bounds((nr, nc), rows, cols)


def beam_tiles(
    src: Position,
    direction: Direction,
    walls: set[Position],
    lasers: set[Position],
    rows: int,
    cols: int,
) -> list[Position]:
    """Tiles a laser beam would cover from src going direction, stopping at walls/lasers."""
    dr, dc = direction.delta
    r, c = src[0] + dr, src[1] + dc
    tiles: list[Position] = []
    while in_bounds((r, c), rows, cols):
        if (r, c) in walls or (r, c) in lasers:
            break
        tiles.append((r, c))
        r += dr
        c += dc
    return tiles


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

WEIGHTS = [w for w, _ in WALL_SHAPES]
SHAPES = [s for _, s in WALL_SHAPES]


def place_wall_shapes(free_cells: list[tuple[int, int]], budget: int, rng: random.Random) -> list[tuple[int, int]]:
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
        for shape in rng.choices(SHAPES, weights=WEIGHTS, k=4):
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
