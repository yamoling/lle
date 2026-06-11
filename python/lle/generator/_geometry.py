"""Pure grid-geometry helpers used by generators.

These helpers stay independent from the solver so generator code can reuse
basic grid logic without importing the SAT machinery.
"""

from __future__ import annotations

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
