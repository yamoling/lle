from __future__ import annotations

from lle.tiles import Direction

from ._candidates import CandidateLayout
from ._geometry import beam_tiles, geometry_ok, points_out_immediately
from .generator import Generator, _LayoutRetry


class RandomGenerator(Generator):
    """Random sampling generator used by `lle.generate(kind="random")`.

    It samples positions, checks the laser geometry, and relies on the SAT solver
    as a final validity check.
    """

    def _sample_unique_positions(self, k: int) -> list[tuple[int, int]]:
        all_pos = [(r, c) for r in range(self.rows) for c in range(self.cols)]
        return self._rng.sample(all_pos, k)

    def _random_direction(self) -> Direction:
        return self._rng.choice([Direction.NORTH, Direction.SOUTH, Direction.EAST, Direction.WEST])

    def _make_candidate_layout(self) -> CandidateLayout:
        total = self.agents + self.agents + self.n_walls + self.n_lasers
        chosen = self._sample_unique_positions(total)
        i = 0
        agents = chosen[i : i + self.agents]
        i += self.agents
        exits = chosen[i : i + self.agents]
        i += self.agents
        walls = chosen[i : i + self.n_walls]
        i += self.n_walls
        laser_pos = chosen[i : i + self.n_lasers]
        lasers = [(k, pos, self._random_direction()) for k, pos in enumerate(laser_pos)]
        layout = CandidateLayout(agents=agents, exits=exits, walls=walls, lasers=lasers)
        if not self._geometry_ok(layout):
            raise _LayoutRetry()
        return layout

    def _geometry_ok(self, layout: CandidateLayout) -> bool:
        return geometry_ok(layout, self.rows, self.cols)
