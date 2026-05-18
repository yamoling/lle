"""Grid helpers used by the solver."""

from __future__ import annotations

from lle.world import World

from .types import Position


def all_positions(world: World) -> list[Position]:
    """Every (i, j) cell in the grid."""
    return [(i, j) for i in range(world.height) for j in range(world.width)]


def is_within_bounds(world: World, pos: Position) -> bool:
    i, j = pos
    return 0 <= i < world.height and 0 <= j < world.width


def get_neighbors(world: World, pos: Position) -> list[Position]:
    """4-directional neighbors that are within bounds."""
    i, j = pos
    result = []
    for di, dj in ((-1, 0), (1, 0), (0, -1), (0, 1)):
        ni, nj = i + di, j + dj
        if 0 <= ni < world.height and 0 <= nj < world.width:
            result.append((ni, nj))
    return result
