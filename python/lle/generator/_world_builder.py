"""
Programmatic builder for lle.World.

Generators use this to place entities on a grid, then call .build()
to get a real lle.World. The LLE string format is an internal detail.
"""

from __future__ import annotations

from lle.tiles import Direction
from lle.world import World

Position = tuple[int, int]


def _dir_letter(direction: Direction) -> str:
    # lle.Direction.name returns a single letter ("N"/"S"/"E"/"W"), unlike
    # stdlib Enum which would return the full member name.
    return direction.name


class WorldBuilder:
    """
    Build an lle.World programmatically.

    Usage:
        world = (
            WorldBuilder(5, 5)
            .add_agent(0, (0, 0))
            .add_agent(1, (0, 4))
            .add_exit((4, 0))
            .add_exit((4, 4))
            .add_wall((2, 2))
            .add_laser(0, (1, 0), Direction.EAST)
            .build()
        )
    """

    def __init__(self, width: int, height: int):
        self.width = width
        self.height = height
        self._grid: list[list[str]] = [["." for _ in range(width)] for _ in range(height)]

    def _check_bounds(self, pos: Position):
        r, c = pos
        if not (0 <= r < self.height and 0 <= c < self.width):
            raise ValueError(f"Position {pos} out of bounds ({self.height}x{self.width})")

    def _check_free(self, pos: Position):
        r, c = pos
        if self._grid[r][c] != ".":
            raise ValueError(f"Position {pos} already occupied by '{self._grid[r][c]}'")

    def add_agent(self, agent_id: int, pos: Position) -> "WorldBuilder":
        self._check_bounds(pos)
        self._check_free(pos)
        self._grid[pos[0]][pos[1]] = f"S{agent_id}"
        return self

    def add_exit(self, pos: Position) -> "WorldBuilder":
        self._check_bounds(pos)
        self._check_free(pos)
        self._grid[pos[0]][pos[1]] = "X"
        return self

    def add_wall(self, pos: Position) -> "WorldBuilder":
        self._check_bounds(pos)
        self._check_free(pos)
        self._grid[pos[0]][pos[1]] = "@"
        return self

    def add_gem(self, pos: Position) -> "WorldBuilder":
        self._check_bounds(pos)
        self._check_free(pos)
        self._grid[pos[0]][pos[1]] = "G"
        return self

    def add_laser(self, agent_id: int, pos: Position, direction: Direction) -> "WorldBuilder":
        self._check_bounds(pos)
        self._check_free(pos)
        self._grid[pos[0]][pos[1]] = f"L{agent_id}{_dir_letter(direction)}"
        return self

    def build(self):
        """Serialize the grid and construct a real lle.World."""
        world_str = "\n".join(" ".join(row) for row in self._grid)
        world = World(world_str)
        world.reset()
        return world
