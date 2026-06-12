from __future__ import annotations

from dataclasses import dataclass

from lle.tiles import Direction
from lle.types import Position

from ._geometry import beam_tiles, points_out_immediately


@dataclass(frozen=True)
class CandidateLayout:
    """ "Candidate layouts sampled by generators before world construction.

    A `CandidateLayout` stores the raw positions chosen by a generator before the
    layout is turned into a `World`.
    """

    height: int
    width: int
    agents: list[tuple[int, int]]
    exits: list[tuple[int, int]]
    walls: list[tuple[int, int]]
    lasers: list[tuple[int, tuple[int, int], Direction]]  # (owner, pos, dir)

    def is_geometry_valid(self):
        wall_set = set(self.walls)
        laser_set = {pos for _, pos, _ in self.lasers}
        exit_set = set(self.exits)
        all_beam: set[Position] = set()
        for _owner, src, direction in self.lasers:
            if points_out_immediately(src, direction, self.height, self.width):
                return False
            tiles = beam_tiles(src, direction, wall_set, laser_set, self.height, self.width)
            if len(tiles) < 2:
                return False
            all_beam.update(tiles)
        if exit_set & all_beam:
            return False
        return True
