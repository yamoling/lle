from __future__ import annotations

from dataclasses import dataclass

from lle.tiles import Direction
from lle.types import Position

from .geometry import beam_tiles, points_out_immediately


@dataclass(frozen=True)
class CandidateLayout:
    """Candidate layouts sampled by generators before world construction.

    A `CandidateLayout` stores the raw positions chosen by a generator before the
    layout is turned into a `World`.
    """

    height: int
    width: int
    agents: list[tuple[int, int]]
    exits: list[tuple[int, int]]
    gems: list[tuple[int, int]]
    walls: list[tuple[int, int]]
    lasers: list[tuple[int, tuple[int, int], Direction]]  # (owner, pos, dir)

    def is_geometry_valid(self):
        """Whether the sampled positions form a layout that can be turned into a `World`.

        A beam may cover neither an exit nor an agent start: the world string writes one token per
        cell, so a beam tile drawn over `S<id>` or `X` erases it and the world fails to parse.
        """
        wall_set = set(self.walls)
        laser_set = {pos for _, pos, _ in self.lasers}
        blocked = set(self.exits) | set(self.agents)
        all_beam: set[Position] = set()
        for _owner, src, direction in self.lasers:
            if points_out_immediately(src, direction, self.height, self.width):
                return False
            tiles = beam_tiles(src, direction, wall_set, laser_set, self.height, self.width)
            if len(tiles) < 2:
                return False
            all_beam.update(tiles)
        if blocked & all_beam:
            return False
        return True
